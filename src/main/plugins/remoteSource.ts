/**
 * 自定义远程插件源（参考 Sileo 的仓库模型）。
 *
 * 地址全部由用户运行时输入并存在设置里，本文件不含任何预置地址——
 * 这正是 PluginSource 抽象要达到的效果：新增一个实现类即可，插件系统本体不动。
 *
 * ── 关于 HTTP 明文 ────────────────────────────────────────────────
 * 按需求支持 http://。必须说清它挡不住什么：明文传输时网络路径上的人可以替换
 * 索引与插件包，而插件是应用内运行的代码，所以这是代码执行级别的风险，
 * 不是「下载损坏」级别。索引里的 sha256 在这种场景下也不构成防护——
 * 攻击者能同时改写索引和包，校验和照样自洽。
 *
 * Sileo 能容忍 HTTP 是因为 APT 仓库有 GPG 签名（Release 文件带签名，
 * 明文传输也能验出篡改）。本项目没有签名体系，因此：
 *   - HTTP 源在界面上标注「明文」，添加时提示风险，由用户自行承担；
 *   - sha256 仍然强制校验，它能抓住传输损坏与仓库放错包这类非对抗性问题；
 *   - 重定向不允许静默切换协议（https→http 降级必须显式失败）。
 * 若日后引入签名，验签应加在 downloadPackage 取到字节之后、返回之前。
 */
import { createHash } from 'node:crypto'
import { request as httpRequest } from 'node:http'
import { request as httpsRequest } from 'node:https'
import type { IncomingMessage } from 'node:http'
import { URL } from 'node:url'
import type { PluginManifest, PluginRepoConfig } from '@common'
import {
  assertAppVersionCompatible,
  validatePluginManifest,
  type PluginDownloadProgress,
  type PluginMeta,
  type PluginSource
} from './source'

/** 索引响应上限 2MB：正常清单几十 KB，超出即视为异常内容。 */
const MAX_REGISTRY_BYTES = 2 * 1024 * 1024
/** 单个插件包上限 64MB。 */
const MAX_PACKAGE_BYTES = 64 * 1024 * 1024
/** 单次请求超时。 */
const TIMEOUT_MS = 20_000
/** 重定向跟随上限。 */
const MAX_REDIRECTS = 5

const SHA256_RE = /^[a-f0-9]{64}$/

/** 由地址推导稳定 id（同一地址每次得到同一个 id，用于去重与 source id）。 */
export function repoIdFromUrl(url: string): string {
  return createHash('sha256').update(url.trim().toLowerCase()).digest('hex').slice(0, 16)
}

/**
 * 校验并规范化用户输入的仓库地址。
 * 只接受 http/https —— 其余协议（file:、data:、javascript: 等）一律拒绝，
 * 否则「远程源」会变成读本地任意文件的通道。
 */
export function normalizeRepoUrl(input: string): { url: string; insecure: boolean } {
  const raw = input.trim()
  if (!raw) throw new Error('请填写仓库地址')
  let parsed: URL
  try {
    // 没写协议时按 http 补全，贴地址时更省事（Sileo 也是这个行为）
    parsed = new URL(/^[a-z][a-z0-9+.-]*:\/\//i.test(raw) ? raw : `http://${raw}`)
  } catch {
    throw new Error('仓库地址格式无效')
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    throw new Error(`不支持的协议：${parsed.protocol}（只接受 http 或 https）`)
  }
  if (!parsed.hostname) throw new Error('仓库地址缺少主机名')
  // 去掉 hash 与凭据，避免把 token 写进设置文件
  parsed.hash = ''
  parsed.username = ''
  parsed.password = ''
  return { url: parsed.toString(), insecure: parsed.protocol === 'http:' }
}

/** 一次 HTTP(S) GET，返回响应体字节。跟随重定向但不允许协议降级。 */
function fetchBytes(
  url: string,
  limit: number,
  onProgress?: PluginDownloadProgress,
  redirectsLeft = MAX_REDIRECTS,
  originProtocol?: string
): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    let target: URL
    try {
      target = new URL(url)
    } catch {
      reject(new Error('地址格式无效'))
      return
    }
    if (target.protocol !== 'http:' && target.protocol !== 'https:') {
      reject(new Error(`不支持的协议：${target.protocol}`))
      return
    }
    // https 起步的请求不允许被重定向到 http：那会让原本加密的链路静默降级
    if (originProtocol === 'https:' && target.protocol === 'http:') {
      reject(new Error('仓库把 https 重定向到了 http（明文降级），已中止'))
      return
    }

    const doRequest = target.protocol === 'https:' ? httpsRequest : httpRequest
    const req = doRequest(
      target,
      {
        method: 'GET',
        headers: {
          // 不带 Cookie / 凭据，纯匿名取公开资源
          accept: '*/*',
          'user-agent': 'kunyin-desktop'
        },
        timeout: TIMEOUT_MS
      },
      (res: IncomingMessage) => {
        const status = res.statusCode ?? 0

        if (status >= 300 && status < 400 && res.headers.location) {
          res.resume()
          if (redirectsLeft <= 0) {
            reject(new Error('重定向次数过多'))
            return
          }
          const next = new URL(res.headers.location, target).toString()
          fetchBytes(next, limit, onProgress, redirectsLeft - 1, originProtocol ?? target.protocol)
            .then(resolve)
            .catch(reject)
          return
        }
        if (status !== 200) {
          res.resume()
          reject(new Error(`仓库返回 HTTP ${status || '无状态码'}`))
          return
        }

        // Content-Length 只用于驱动进度条，不能作为大小上限的依据——
        // 它由服务端给出，可以谎报。真正的上限靠累计字节数强制。
        const declared = Number(res.headers['content-length'])
        const total = Number.isFinite(declared) && declared > 0 ? declared : 0
        const chunks: Buffer[] = []
        let received = 0

        res.on('data', (chunk: Buffer) => {
          received += chunk.length
          if (received > limit) {
            req.destroy()
            reject(new Error(`响应体超过上限 ${Math.round(limit / 1024 / 1024)}MB`))
            return
          }
          chunks.push(chunk)
          if (total) onProgress?.(Math.min(received / total, 1))
        })
        res.on('end', () => {
          onProgress?.(1)
          resolve(Buffer.concat(chunks))
        })
        res.on('error', reject)
      }
    )
    req.on('timeout', () => {
      req.destroy(new Error(`请求超时（${TIMEOUT_MS / 1000}s）`))
    })
    req.on('error', (err) => {
      reject(new Error(`连接仓库失败：${err.message}`))
    })
    req.end()
  })
}

/** 索引条目里必须带 sha256 与下载地址，与包内 manifest 的校验规则复用同一套。 */
function parseRegistryEntry(value: unknown, base: string, sourceId: string): PluginMeta {
  const manifest = validatePluginManifest(value, sourceId)
  const item = value as Record<string, unknown>

  const rawUrl = typeof item.downloadUrl === 'string' ? item.downloadUrl.trim() : ''
  if (!rawUrl) throw new Error(`插件 ${manifest.id} 的索引条目缺少 downloadUrl`)
  // 相对地址按索引地址解析，仓库可以只写 packages/foo.zip
  let resolved: URL
  try {
    resolved = new URL(rawUrl, base)
  } catch {
    throw new Error(`插件 ${manifest.id} 的 downloadUrl 无效：${rawUrl}`)
  }
  if (resolved.protocol !== 'http:' && resolved.protocol !== 'https:') {
    throw new Error(`插件 ${manifest.id} 的 downloadUrl 协议不受支持：${resolved.protocol}`)
  }

  // 强制校验和：远程包必须给出期望值，否则装进来的字节没有任何依据可比
  const sha = typeof item.sha256 === 'string' ? item.sha256.trim().toLowerCase() : ''
  if (!sha) throw new Error(`插件 ${manifest.id} 的索引条目缺少 sha256`)
  if (!SHA256_RE.test(sha)) throw new Error(`插件 ${manifest.id} 的 sha256 格式无效`)

  const size = typeof item.sizeBytes === 'number' && item.sizeBytes > 0 ? item.sizeBytes : undefined

  return {
    ...manifest,
    // 这里的 downloadUrl 是真实可访问地址（本类内部使用），与本地源的 `local:<id>` 不同
    downloadUrl: resolved.toString(),
    sha256: sha,
    sizeBytes: size
  }
}

/**
 * 远程仓库源。一个实例对应用户添加的一个地址。
 */
export class RemoteSource implements PluginSource {
  readonly id: string
  readonly label: string
  readonly insecure: boolean
  private readonly url: string
  /** 上次拉取到的条目，download 时按 id 找回真实地址与期望校验和 */
  private cache = new Map<string, PluginMeta>()

  constructor(config: PluginRepoConfig) {
    const { url, insecure } = normalizeRepoUrl(config.url)
    this.url = url
    this.insecure = insecure
    this.id = `repo-${config.id}`
    this.label = config.name.trim() || new URL(url).hostname
  }

  async fetchRegistry(): Promise<PluginMeta[]> {
    const bytes = await fetchBytes(this.url, MAX_REGISTRY_BYTES, undefined, MAX_REDIRECTS)
    let doc: unknown
    try {
      doc = JSON.parse(bytes.toString('utf8'))
    } catch {
      throw new Error('仓库索引不是合法 JSON')
    }
    if (!doc || typeof doc !== 'object') throw new Error('仓库索引必须是 JSON 对象')
    const list = (doc as { plugins?: unknown }).plugins
    if (!Array.isArray(list)) throw new Error('仓库索引缺少 plugins 数组')

    const metas: PluginMeta[] = []
    const errors: string[] = []
    for (const entry of list) {
      try {
        metas.push(parseRegistryEntry(entry, this.url, this.id))
      } catch (err) {
        // 单条坏掉不该让整个仓库不可用，记下来继续
        errors.push(err instanceof Error ? err.message : String(err))
      }
    }
    if (!metas.length && errors.length) {
      throw new Error(`索引内没有可用条目：${errors[0]}`)
    }
    this.cache = new Map(metas.map((m) => [m.id, m]))
    return metas
  }

  async download(id: string, onProgress?: PluginDownloadProgress): Promise<Buffer> {
    const meta = this.cache.get(id)
    if (!meta) throw new Error('请先刷新仓库列表')
    const expected = meta.sha256
    if (!expected || !SHA256_RE.test(expected)) {
      throw new Error('索引条目缺少有效的 sha256，拒绝下载')
    }

    const bytes = await fetchBytes(meta.downloadUrl, MAX_PACKAGE_BYTES, onProgress, MAX_REDIRECTS)
    const actual = createHash('sha256').update(bytes).digest('hex')
    if (actual !== expected) {
      throw new Error('插件包 SHA-256 与索引不一致（文件已损坏或被替换）')
    }
    return bytes
  }

  /** 取索引里声明的期望校验和（安装记录用）。 */
  expectedChecksum(id: string): string | undefined {
    return this.cache.get(id)?.sha256
  }

  /** 校验某条目与当前应用版本兼容（供安装前复查）。 */
  assertCompatible(id: string, appVersion: string): void {
    const meta = this.cache.get(id)
    if (!meta) throw new Error('请先刷新仓库列表')
    assertAppVersionCompatible(meta, appVersion)
  }

  /** 已缓存的条目（listPlugins 合并用）。 */
  cachedManifest(id: string): PluginManifest | undefined {
    return this.cache.get(id)
  }
}
