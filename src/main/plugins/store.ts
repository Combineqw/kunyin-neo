/**
 * 插件安装状态管理。
 *
 * 数据源经 PluginSource 抽象（见 source.ts），当前只注册 LocalSource，
 * 因此本文件不含任何远程地址；日后接在线商店只需新增数据源实现。
 *
 * 与旧版的关键差异：已安装插件的清单随安装状态一起持久化。旧版每次
 * 卸载/启用都要重新拉远端清单才能拿到 manifest，离线时连"停用"都做不到。
 */
import { mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises'
import { join, resolve, sep } from 'node:path'
import { app } from 'electron'
import { appDataPath } from '../core/paths'
import {
  assertAppVersionCompatible,
  localSource,
  validatePluginManifest,
  type PluginMeta
} from './source'
import { enabledSources, noteFetch } from './repos'
import type {
  InstalledPluginState,
  PluginActionResult,
  PluginStoreEvent,
  PluginStoreItem,
  PluginStoreResult
} from '@common'

const PLUGIN_DATA_DIR = 'plugins'
const INDEX_FILE = 'index.json'

/** 持久化条目 = 安装状态 + 该插件的清单快照。 */
interface InstalledRecord extends InstalledPluginState {
  manifest: PluginMeta
}

function indexPath(): string {
  return appDataPath(PLUGIN_DATA_DIR, INDEX_FILE)
}

function pluginRoot(id: string): string {
  if (!/^[a-zA-Z0-9._-]+$/.test(id) || id === '.' || id === '..') {
    throw new Error('插件 ID 无效')
  }
  const root = resolve(appDataPath(PLUGIN_DATA_DIR, id))
  const parent = resolve(appDataPath(PLUGIN_DATA_DIR)) + sep
  if (!root.startsWith(parent)) throw new Error('插件路径无效')
  return root
}

function isInstalledRecord(value: unknown): value is InstalledRecord {
  if (!value || typeof value !== 'object') return false
  const item = value as Partial<InstalledRecord>
  return (
    typeof item.id === 'string' &&
    typeof item.version === 'string' &&
    typeof item.installedAt === 'number' &&
    typeof item.enabled === 'boolean' &&
    typeof item.packagePath === 'string' &&
    !!item.manifest &&
    typeof item.manifest === 'object'
  )
}

async function readInstalled(): Promise<InstalledRecord[]> {
  try {
    const raw = JSON.parse(await readFile(indexPath(), 'utf8')) as unknown
    if (!Array.isArray(raw)) return []
    return raw.filter(isInstalledRecord).map((item) => ({
      ...item,
      manifest: {
        // 重新过一遍字段校验，非法记录不进入界面
        ...validatePluginManifest(item.manifest, 'local'),
        // 校验和与体积是安装时按真实字节算出的事实，不在 manifest 校验范围内，
        // 需要从持久化记录里带回来（否则每次读取都会被抹成 undefined）
        sha256: typeof item.manifest.sha256 === 'string' ? item.manifest.sha256 : undefined,
        sizeBytes: typeof item.manifest.sizeBytes === 'number' ? item.manifest.sizeBytes : undefined
      }
    }))
  } catch {
    return []
  }
}

async function writeInstalled(items: InstalledRecord[]): Promise<void> {
  const dir = appDataPath(PLUGIN_DATA_DIR)
  await mkdir(dir, { recursive: true })
  const tmp = `${indexPath()}.tmp`
  await writeFile(tmp, JSON.stringify(items, null, 2), 'utf8')
  await rename(tmp, indexPath())
}

function toStoreItem(record: InstalledRecord): PluginStoreItem {
  return {
    ...record.manifest,
    installed: true,
    enabled: record.enabled,
    installedVersion: record.version,
    installedAt: record.installedAt
  }
}

function pendingItem(manifest: PluginMeta): PluginStoreItem {
  return { ...manifest, installed: false, enabled: false }
}

/**
 * 列出插件：已安装的（来自本地索引）+ 本地待装的 + 各启用中远程源的条目。
 *
 * 远程源逐个拉取，单个源失败不影响其余源与本地功能——错误记进该源的状态里，
 * 由界面展示。已安装条目优先，同 id 的远程条目不再重复列出。
 */
export async function listPlugins(): Promise<PluginStoreResult> {
  const [installed, staged, remotes] = await Promise.all([
    readInstalled(),
    localSource.fetchRegistry(),
    enabledSources()
  ])
  const seen = new Set(installed.map((item) => item.id))
  const plugins: PluginStoreItem[] = [...installed.map(toStoreItem)]

  for (const meta of staged) {
    if (seen.has(meta.id)) continue
    seen.add(meta.id)
    plugins.push(pendingItem(meta))
  }

  for (const source of remotes) {
    try {
      const metas = await source.fetchRegistry()
      noteFetch(source.id.replace(/^repo-/, ''), { count: metas.length })
      for (const meta of metas) {
        if (seen.has(meta.id)) continue
        seen.add(meta.id)
        plugins.push(pendingItem(meta))
      }
    } catch (err) {
      noteFetch(source.id.replace(/^repo-/, ''), {
        error: err instanceof Error ? err.message : String(err)
      })
    }
  }

  const remoteLabel = remotes.length ? `，${remotes.length} 个自定义源` : ''
  return {
    plugins,
    // 来源标识而非 URL：界面只用它做展示
    sourceLabel: `${localSource.label}${remoteLabel}`,
    fetchedAt: Date.now()
  }
}

function actionResult(plugin: PluginStoreItem, message: string): PluginActionResult {
  return { plugin, message }
}

/** 选择/拖入一个本地 zip：解析、校验、暂存，返回其清单条目。 */
export async function stageLocalPackage(filePath: string): Promise<PluginActionResult> {
  const manifest = await localSource.stageFile(filePath, app.getVersion())
  return actionResult(pendingItem(manifest), `已读取插件包：${manifest.name} ${manifest.version}`)
}

/**
 * 安装插件。id 可能来自本地暂存，也可能来自某个远程源——
 * 先在本地找，找不到再问各启用中的源，谁认这个 id 就由谁提供字节。
 * 远程源的 download() 内部已按索引里的 sha256 逐字节校验过。
 */
export async function installPlugin(id: string): Promise<PluginActionResult> {
  const localManifests = await localSource.fetchRegistry()
  let manifest = localManifests.find((item) => item.id === id)
  let bytes: Buffer | undefined
  let fromLocal = false

  if (manifest) {
    fromLocal = true
    assertAppVersionCompatible(manifest, app.getVersion())
    bytes = await localSource.download(id)
  } else {
    for (const source of await enabledSources()) {
      const cached = source.cachedManifest(id)
      if (!cached) continue
      // 安装前复查兼容性（列表拉取与点安装之间应用可能被降级覆盖安装）
      source.assertCompatible(id, app.getVersion())
      manifest = cached as PluginMeta
      bytes = await source.download(id)
      break
    }
  }

  if (!manifest || !bytes) throw new Error('插件包不存在或已过期，请刷新列表后重试')
  const root = pluginRoot(id)
  await mkdir(root, { recursive: true })
  const packagePath = join(root, `${id}.zip`)
  const tmpPath = `${packagePath}.tmp`
  await writeFile(tmpPath, bytes)
  await rename(tmpPath, packagePath)

  const installed = await readInstalled()
  const record: InstalledRecord = {
    id,
    version: manifest.version,
    installedAt: Date.now(),
    enabled: true,
    packagePath,
    manifest
  }
  await writeInstalled([...installed.filter((item) => item.id !== id), record])
  // 只有本地源需要释放暂存字节；远程源的缓存里只有清单，不占内存
  if (fromLocal) localSource.unstage(id)
  return actionResult(toStoreItem(record), '插件已安装')
}

export async function uninstallPlugin(id: string): Promise<PluginActionResult> {
  const installed = await readInstalled()
  const record = installed.find((item) => item.id === id)
  if (!record) throw new Error('插件尚未安装')
  await rm(pluginRoot(id), { recursive: true, force: true })
  await writeInstalled(installed.filter((item) => item.id !== id))
  return actionResult(pendingItem(record.manifest), '插件已卸载')
}

export async function setPluginEnabled(id: string, enabled: boolean): Promise<PluginActionResult> {
  const installed = await readInstalled()
  const current = installed.find((item) => item.id === id)
  if (!current) throw new Error('请先安装插件')
  if (enabled) assertAppVersionCompatible(current.manifest, app.getVersion())
  const next: InstalledRecord = { ...current, enabled }
  await writeInstalled(installed.map((item) => (item.id === id ? next : item)))
  return actionResult(toStoreItem(next), enabled ? '插件已启用' : '插件已停用')
}

export function toPluginEvent(
  type: PluginStoreEvent['type'],
  result: PluginActionResult
): PluginStoreEvent {
  return { type, plugin: result.plugin }
}
