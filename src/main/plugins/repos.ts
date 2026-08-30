/**
 * 用户添加的插件源清单。
 *
 * 存在 appData/plugin-repos.json，与设置文件分开：源清单是插件系统的资产，
 * 跟着插件目录一起备份/迁移比塞进 settings.json 更合理。
 * 文件里只有用户自己粘进来的地址，代码中没有任何预置地址。
 */
import { mkdir, readFile, writeFile, rename } from 'node:fs/promises'
import { dirname } from 'node:path'
import { appDataPath } from '../core/paths'
import type { PluginRepoConfig, PluginRepoStatus } from '@common'
import { RemoteSource, normalizeRepoUrl, repoIdFromUrl } from './remoteSource'

function reposPath(): string {
  return appDataPath('plugins', 'repos.json')
}

function isRepoRecord(value: unknown): value is PluginRepoConfig {
  if (!value || typeof value !== 'object') return false
  const v = value as Record<string, unknown>
  return (
    typeof v.id === 'string' &&
    typeof v.url === 'string' &&
    typeof v.name === 'string' &&
    typeof v.addedAt === 'number' &&
    typeof v.enabled === 'boolean'
  )
}

/** 最近一次拉取结果，仅内存态（重启后重新拉） */
const runtime = new Map<string, { error?: string; fetchedAt?: number; count?: number }>()

/** 已构造的源实例，按 repo id 缓存（保住 fetchRegistry 填进去的条目缓存） */
const instances = new Map<string, RemoteSource>()

export async function readRepos(): Promise<PluginRepoConfig[]> {
  try {
    const raw = JSON.parse(await readFile(reposPath(), 'utf8'))
    if (!Array.isArray(raw)) return []
    return raw.filter(isRepoRecord).map((item) => ({
      ...item,
      // 地址每次读回都重新规范化：手改过文件也不会把非法协议带进来
      url: normalizeRepoUrl(item.url).url
    }))
  } catch {
    // 文件不存在或损坏都按「没有源」处理，不影响本地安装
    return []
  }
}

async function writeRepos(items: PluginRepoConfig[]): Promise<void> {
  const path = reposPath()
  // plugins/ 首次运行时可能还不存在（未装过任何插件）
  await mkdir(dirname(path), { recursive: true })
  const tmp = `${path}.tmp`
  await writeFile(tmp, JSON.stringify(items, null, 2), 'utf8')
  await rename(tmp, path)
}

/** 取某个源的实例（缺失则按配置构造）。 */
export async function sourceForRepo(id: string): Promise<RemoteSource | undefined> {
  const cached = instances.get(id)
  if (cached) return cached
  const repos = await readRepos()
  const config = repos.find((r) => r.id === id)
  if (!config) return undefined
  const source = new RemoteSource(config)
  instances.set(id, source)
  return source
}

/** 全部启用中的源实例。 */
export async function enabledSources(): Promise<RemoteSource[]> {
  const repos = await readRepos()
  const out: RemoteSource[] = []
  for (const config of repos.filter((r) => r.enabled)) {
    let source = instances.get(config.id)
    if (!source) {
      source = new RemoteSource(config)
      instances.set(config.id, source)
    }
    out.push(source)
  }
  return out
}

/** 添加一个源。地址重复时报错而不是静默去重，让用户知道已经加过。 */
export async function addRepo(url: string, name: string): Promise<PluginRepoStatus> {
  const normalized = normalizeRepoUrl(url)
  const id = repoIdFromUrl(normalized.url)
  const repos = await readRepos()
  if (repos.some((r) => r.id === id)) throw new Error('这个仓库已经添加过了')

  const config: PluginRepoConfig = {
    id,
    url: normalized.url,
    name: name.trim(),
    addedAt: Date.now(),
    enabled: true
  }
  await writeRepos([...repos, config])
  const source = new RemoteSource(config)
  instances.set(id, source)

  // 立即试拉一次：地址写错、索引格式不对要当场告诉用户，而不是等下次刷新
  try {
    const metas = await source.fetchRegistry()
    runtime.set(id, { fetchedAt: Date.now(), count: metas.length })
  } catch (err) {
    runtime.set(id, { error: err instanceof Error ? err.message : String(err) })
  }
  return toStatus(config, normalized.insecure)
}

export async function removeRepo(id: string): Promise<void> {
  const repos = await readRepos()
  if (!repos.some((r) => r.id === id)) throw new Error('这个仓库不存在')
  await writeRepos(repos.filter((r) => r.id !== id))
  instances.delete(id)
  runtime.delete(id)
}

export async function setRepoEnabled(id: string, enabled: boolean): Promise<PluginRepoStatus> {
  const repos = await readRepos()
  const config = repos.find((r) => r.id === id)
  if (!config) throw new Error('这个仓库不存在')
  const next = { ...config, enabled }
  await writeRepos(repos.map((r) => (r.id === id ? next : r)))
  return toStatus(next, normalizeRepoUrl(next.url).insecure)
}

/** 记录一次拉取结果（listPlugins 调用）。 */
export function noteFetch(id: string, result: { error?: string; count?: number }): void {
  runtime.set(id, { ...result, fetchedAt: Date.now() })
}

function toStatus(config: PluginRepoConfig, insecure: boolean): PluginRepoStatus {
  const rt = runtime.get(config.id)
  return {
    ...config,
    insecure,
    lastError: rt?.error,
    lastFetchedAt: rt?.fetchedAt,
    pluginCount: rt?.count
  }
}

/** 界面用的源列表（含明文标记与最近拉取状态）。 */
export async function listRepos(): Promise<PluginRepoStatus[]> {
  const repos = await readRepos()
  return repos.map((config) => toStatus(config, normalizeRepoUrl(config.url).insecure))
}
