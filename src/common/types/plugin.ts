/**
 * 插件清单与本地安装状态。
 * 插件包只作为受控资源记录，不在 renderer 中直接执行。
 * 当前仅有本地数据源（LocalSource），类型层不含任何远程地址。
 */
export type PluginKind = 'lyrics' | 'theme' | 'visual' | 'other'

export interface PluginManifest {
  id: string
  name: string
  version: string
  description: string
  author: string
  kind: PluginKind
  /** 数据源内部定位标识（本地源为 `local:<id>`）。不是可访问的 URL，界面不得直接打开。 */
  downloadUrl: string
  homepage?: string
  iconUrl?: string
  sha256?: string
  sizeBytes?: number
  /** 最低应用版本：具体 semver 版本（视作 >=）或 semver 范围 */
  minAppVersion?: string
}

export interface InstalledPluginState {
  id: string
  version: string
  installedAt: number
  enabled: boolean
  packagePath: string
}

export interface PluginStoreItem extends PluginManifest {
  installed: boolean
  enabled: boolean
  installedVersion?: string
  installedAt?: number
}

export interface PluginStoreResult {
  plugins: PluginStoreItem[]
  /** 当前数据源的展示名（如「本地安装」）。是 PluginSource.label，不是地址。 */
  sourceLabel: string
  fetchedAt: number
}

export interface PluginActionResult {
  plugin: PluginStoreItem
  message: string
}

export interface PluginStoreEvent {
  type: 'installed' | 'uninstalled' | 'enabled' | 'disabled'
  plugin: PluginStoreItem
}

/**
 * 批量清单文档结构：`{ plugins: [...] }`。
 * 供 PluginSource.fetchRegistry() 的实现解析清单用；
 * 日后新增在线源实现类时直接复用，无需改动本类型。
 */
export interface PluginRepositoryDocument {
  plugins: PluginManifest[]
}

/**
 * 用户添加的自定义插件源（参考 Sileo 的仓库模型：粘贴地址→拉索引→列包→装）。
 *
 * 地址由用户运行时输入并持久化到设置，代码里没有任何预置地址。
 */
export interface PluginRepoConfig {
  /** 由地址推导的稳定标识（host+path 的 sha256 前 16 位），作为 source id 后缀 */
  id: string
  /** 仓库索引地址（http 或 https）。包地址按此地址做相对解析。 */
  url: string
  /** 展示名。用户可留空，此时取 host */
  name: string
  /** 添加时间 */
  addedAt: number
  /** 是否参与列表拉取 */
  enabled: boolean
}

/** 源在界面上的展示状态 */
export interface PluginRepoStatus extends PluginRepoConfig {
  /** true 表示地址是 http:// 明文传输 */
  insecure: boolean
  /** 最近一次拉取的结果；未拉过为 undefined */
  lastError?: string
  lastFetchedAt?: number
  pluginCount?: number
}
