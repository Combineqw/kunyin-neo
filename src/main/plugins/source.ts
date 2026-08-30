/**
 * 插件数据源抽象。
 *
 * 目前只实现 LocalSource（本地 zip 安装）。日后上线在线商店只需新增一个
 * PluginSource 实现类并在 registry 里注册，插件系统本体（安装/启用/停用/卸载/
 * 版本兼容检查）无需改动——因此本文件与 store.ts 中不允许出现任何写死的远程地址。
 */
import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { satisfies, valid, validRange } from 'semver'
import type { PluginManifest } from '@common'
import { readZipFile } from './zip'

/** 下载进度回调（0~1）。在线源实现时用于驱动进度条。 */
export type PluginDownloadProgress = (ratio: number) => void

/**
 * 插件数据源。fetchRegistry 给出可见清单，download 取回插件包字节。
 * 两个方法都不假设传输方式，本地源即读文件，在线源即发请求。
 */
export interface PluginSource {
  /** 数据源标识，用于界面展示与来源区分 */
  readonly id: string
  /** 面向用户的来源名称 */
  readonly label: string
  fetchRegistry(): Promise<PluginMeta[]>
  download(id: string, onProgress?: PluginDownloadProgress): Promise<Buffer>
}

/** 清单条目。与 PluginManifest 同构，但不含任何下载地址——地址是数据源的内部实现细节。 */
export type PluginMeta = PluginManifest

/** 插件包内 manifest.json 的解析结果。 */
export interface LocalPackage {
  manifest: PluginMeta
  bytes: Buffer
  sha256: string
  /** 是否比对过外部给出的期望校验和（无期望值时为 false，仅记录实际值） */
  verified: boolean
}

/** 十六进制 sha256（64 位小写） */
const SHA256_RE = /^[a-f0-9]{64}$/

/**
 * 解析校验和文本。兼容 `sha256sum` 的输出格式（`<hash>  <filename>`）与裸 hash。
 * 解析失败返回 undefined，由调用方决定是否报错。
 */
export function parseSha256Text(text: string): string | undefined {
  const token = text.trim().split(/\s+/)[0]?.toLowerCase()
  return token && SHA256_RE.test(token) ? token : undefined
}

const KINDS = ['lyrics', 'theme', 'visual', 'other'] as const

/** 校验插件包内的 manifest.json。字段与内置 PluginManifest 一致，白名单取用。 */
export function validatePluginManifest(value: unknown, sourceId: string): PluginMeta {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('插件清单必须是 JSON 对象')
  }
  const item = value as Record<string, unknown>
  const text = (key: string): string => {
    const v = item[key]
    if (typeof v !== 'string' || !v.trim()) throw new Error(`插件清单缺少有效字段：${key}`)
    return v.trim()
  }
  const id = text('id')
  // id 会作为目录名落盘，白名单之外一律拒绝；'.' 与 '..' 虽在白名单内但是路径别名
  if (!/^[a-zA-Z0-9._-]+$/.test(id) || id === '.' || id === '..') {
    throw new Error(`插件 ID 无效：${id}`)
  }
  const version = text('version')
  if (!valid(version)) throw new Error(`插件版本号不是合法的 semver：${version}`)
  const kind = text('kind')
  if (!(KINDS as readonly string[]).includes(kind)) throw new Error(`插件类型不受支持：${kind}`)

  const minAppVersion = typeof item.minAppVersion === 'string' ? item.minAppVersion.trim() : ''
  if (minAppVersion && !valid(minAppVersion) && !validRange(minAppVersion)) {
    throw new Error(`minAppVersion 不是合法的 semver 版本或范围：${minAppVersion}`)
  }

  return {
    id,
    name: text('name'),
    version,
    description: typeof item.description === 'string' ? item.description.trim() : '',
    author: typeof item.author === 'string' ? item.author.trim() : '',
    kind: kind as PluginMeta['kind'],
    // 本地源没有下载地址；保留字段兼容既有类型，值为来源标识而非 URL
    downloadUrl: `${sourceId}:${id}`,
    // sha256 / sizeBytes 刻意不从包内 manifest 取值：
    // 二者描述的是「整个 zip」，而 manifest.json 就在这个 zip 里面，
    // 包内自述的校验和是个无法构造的不动点（写进去就改变了它自己）。
    // 期望值必须来自包外——本地源读同名 .sha256 旁挂文件，在线源用清单条目。
    // 这里留空，实际值在 stageBuffer 里按真实字节填。
    sha256: undefined,
    sizeBytes: undefined,
    minAppVersion: minAppVersion || undefined
  }
}

/** 应用版本兼容检查。minAppVersion 可为具体版本（视作 >=）或 semver 范围。 */
export function assertAppVersionCompatible(manifest: PluginMeta, appVersion: string): void {
  const required = manifest.minAppVersion
  if (!required) return
  const range = validRange(required) && !valid(required) ? required : `>=${required}`
  // 预发布版本（1.0.8-beta.1 之类）也参与比较，否则 beta 装不上任何插件
  if (!satisfies(appVersion, range, { includePrerelease: true })) {
    throw new Error(`插件需要坤音neo ${required} 或更高版本（当前 ${appVersion}）`)
  }
}

/** 读取 zip 旁挂的 .sha256 文件（不存在或格式不对则返回 undefined）。 */
async function readSidecarSha256(zipPath: string): Promise<string | undefined> {
  const candidates = [`${zipPath}.sha256`, zipPath.replace(/\.zip$/i, '.sha256')]
  for (const candidate of candidates) {
    try {
      const parsed = parseSha256Text(await readFile(candidate, 'utf8'))
      if (parsed) return parsed
    } catch {
      // 旁挂文件是可选的，读不到就继续
    }
  }
  return undefined
}

/**
 * 本地 zip 数据源。清单来自"用户已拖入/选择的包"，因此 fetchRegistry 返回的是
 * 本会话内已解析过的本地包，不产生任何网络请求。
 */
export class LocalSource implements PluginSource {
  readonly id = 'local'
  readonly label = '本地安装'

  /** 已解析待安装的本地包（按 id 暂存，install 时取用） */
  private readonly staged = new Map<string, LocalPackage>()

  async fetchRegistry(): Promise<PluginMeta[]> {
    // 本地源没有远端清单：可见条目就是已暂存的本地包
    return [...this.staged.values()].map((pkg) => pkg.manifest)
  }

  async download(id: string, onProgress?: PluginDownloadProgress): Promise<Buffer> {
    const pkg = this.staged.get(id)
    if (!pkg) throw new Error('本地插件包不存在或已过期，请重新选择文件')
    onProgress?.(1)
    return pkg.bytes
  }

  /**
   * 解析一个本地 zip 文件并暂存，返回其清单。
   *
   * 期望校验和来自包外的旁挂文件：`demo.zip` 旁的 `demo.zip.sha256`
   * （或 `demo.sha256`），格式兼容 `sha256sum` 输出。存在即强校验，
   * 不存在则只记录实际值——本地拖入的包由用户自己保证来源。
   */
  async stageFile(filePath: string, appVersion: string): Promise<PluginMeta> {
    if (!/\.zip$/i.test(filePath)) throw new Error('插件包必须是 .zip 文件')
    const bytes = await readFile(filePath)
    return this.stageBuffer(bytes, appVersion, await readSidecarSha256(filePath))
  }

  /**
   * 解析 zip 字节并暂存（供拖入场景直接传字节）。
   * @param expectedSha256 包外给出的期望校验和；在线源应传清单条目里的值。
   */
  async stageBuffer(
    bytes: Buffer,
    appVersion: string,
    expectedSha256?: string
  ): Promise<PluginMeta> {
    const entry = readZipFile(bytes, 'manifest.json')
    if (!entry) throw new Error('插件包内缺少 manifest.json')
    let raw: unknown
    try {
      raw = JSON.parse(entry.toString('utf8'))
    } catch {
      throw new Error('插件包内的 manifest.json 不是合法 JSON')
    }
    const manifest = validatePluginManifest(raw, this.id)

    // 完整性校验：只认包外给出的期望值
    const sha256 = createHash('sha256').update(bytes).digest('hex')
    const expected = expectedSha256?.trim().toLowerCase()
    if (expected) {
      if (!SHA256_RE.test(expected)) throw new Error('期望的 SHA-256 校验和格式无效')
      if (expected !== sha256) {
        throw new Error('插件包 SHA-256 校验失败（文件可能已损坏或被篡改）')
      }
    }
    // 兼容性检查放在暂存阶段，用户选文件时即刻得到反馈
    assertAppVersionCompatible(manifest, appVersion)

    // 校验和与体积按真实字节回填，供界面展示与安装记录
    const resolved: PluginMeta = { ...manifest, sha256, sizeBytes: bytes.byteLength }
    this.staged.set(resolved.id, { manifest: resolved, bytes, sha256, verified: !!expected })
    return resolved
  }

  /** 取暂存包的校验和（安装后记录用）。 */
  stagedChecksum(id: string): string | undefined {
    return this.staged.get(id)?.sha256
  }

  /** 该暂存包是否比对过包外的期望校验和。 */
  isStagedVerified(id: string): boolean {
    return this.staged.get(id)?.verified ?? false
  }

  /** 安装完成后释放字节，避免长期占内存。 */
  unstage(id: string): void {
    this.staged.delete(id)
  }
}

/** 当前启用的数据源。新增在线源时在此注册即可，其余代码不必改。 */
export const localSource = new LocalSource()
export const pluginSources: PluginSource[] = [localSource]
