/**
 * 应用设置的 JSON 存储（原子写：临时文件 + rename，参照 lx-music-desktop 的 Store）。
 *
 * 存于 `userData/data/settings.json`。敏感凭据不走这里，后续用 safeStorage 单独加密。
 */
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs'
import { DEFAULT_SETTINGS, type AppSettings, type DeepPartial } from '@common'
import { appDataPath } from '../core/paths'

let cache: AppSettings | null = null

function filePath(): string {
  return appDataPath('settings.json')
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}

function clampSetting(value: unknown, fallback: number, min: number, max: number): number {
  const numberValue = typeof value === 'number' && Number.isFinite(value) ? value : fallback
  return Math.max(min, Math.min(max, numberValue))
}

/** 深合并：以 base 为骨架，用 patch 覆盖（缺省项保留 base） */
function deepMerge(base: unknown, patch: unknown): unknown {
  if (!isObject(base) || !isObject(patch)) return patch === undefined ? base : patch
  const out: Record<string, unknown> = { ...base }
  for (const key of Object.keys(patch)) {
    const bv = base[key]
    const pv = patch[key]
    out[key] = isObject(bv) && isObject(pv) ? deepMerge(bv, pv) : pv
  }
  return out
}

function load(): AppSettings {
  try {
    const raw = readFileSync(filePath(), 'utf-8')
    const parsed = JSON.parse(raw) as Partial<AppSettings>
    const merged = deepMerge(DEFAULT_SETTINGS, parsed) as AppSettings
    // 已移除的桌面歌词描边字段不再带入运行时或后续持久化文件。
    delete (merged.lyrics as unknown as Record<string, unknown>).desktopShadowColor
    // 旧字段 writeLyricMeta 迁移到 embedLyric（歌词写入标签拆分出翻译/罗马音/逐字子开关）。
    const dl = merged.download as unknown as Record<string, unknown>
    if (typeof dl.writeLyricMeta === 'boolean') {
      dl.embedLyric = dl.writeLyricMeta
      delete dl.writeLyricMeta
    }
    // v1 → v2：整专曲目号前缀改为默认开启（v1 时该项无界面入口，存的 false 均为旧默认值）。
    if ((parsed.version ?? 0) < 2) merged.download.trackNumberPrefix = true
    // v2 → v3：动态 EQ 配置由默认值补齐；旧十段配置保持可用。
    if ((parsed.version ?? 0) < 3) {
      merged.player.equalizerFilters = []
      merged.player.equalizerPreampDb = 0
      merged.player.equalizerCustomProfiles = []
    }
    // v3 → 当前：SRS 风格音效字段由默认值补齐，默认关闭，避免改变旧用户的原声行为。
    merged.player.srsEnabled = !!merged.player.srsEnabled
    merged.player.srsIntensity = clampSetting(
      merged.player.srsIntensity,
      DEFAULT_SETTINGS.player.srsIntensity,
      0,
      100
    )
    merged.player.srsBass = clampSetting(merged.player.srsBass, DEFAULT_SETTINGS.player.srsBass, 0, 100)
    merged.player.srsVoice = clampSetting(merged.player.srsVoice, DEFAULT_SETTINGS.player.srsVoice, 0, 100)
    merged.player.srsTreble = clampSetting(merged.player.srsTreble, DEFAULT_SETTINGS.player.srsTreble, 0, 100)
    merged.player.srsSpace = clampSetting(merged.player.srsSpace, DEFAULT_SETTINGS.player.srsSpace, 0, 100)
    merged.player.srsLimiter = merged.player.srsLimiter !== false
    merged.player.irsEnabled = merged.player.irsEnabled === true
    const rawPlayer = (parsed.player ?? {}) as Record<string, unknown>
    merged.player.volume = clampSetting(
      merged.player.volume,
      DEFAULT_SETTINGS.player.volume,
      0,
      1
    )
    const hasWetPercent = typeof rawPlayer.irsWetPercent === 'number'
    const legacyDry = clampSetting(
      merged.player.irsDryPercent,
      DEFAULT_SETTINGS.player.irsDryPercent,
      0,
      100
    )
    merged.player.irsWetPercent = clampSetting(
      hasWetPercent ? merged.player.irsWetPercent : 100 - legacyDry,
      DEFAULT_SETTINGS.player.irsWetPercent,
      0,
      100
    )
    // 保留旧字段的镜像值，避免旧备份或外部读取者看到互相矛盾的干湿比例。
    merged.player.irsDryPercent = 100 - merged.player.irsWetPercent
    const profiles = Array.isArray(merged.player.irsProfiles) ? merged.player.irsProfiles : []
    merged.player.irsProfiles = profiles.filter((profile) => profile.id !== 'builtin-srs-normal-headphone')
    merged.player.irsProfileId = typeof merged.player.irsProfileId === 'string' ? merged.player.irsProfileId : ''
    if (!merged.player.irsProfiles.some((profile) => profile.id === merged.player.irsProfileId)) {
      merged.player.irsProfileId = ''
      merged.player.irsEnabled = false
    }
    merged.version = DEFAULT_SETTINGS.version
    return merged
  } catch {
    return { ...DEFAULT_SETTINGS }
  }
}

function persist(settings: AppSettings): void {
  const path = filePath()
  const dir = appDataPath()
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  const tmp = `${path}.tmp`
  writeFileSync(tmp, JSON.stringify(settings, null, 2), 'utf-8')
  renameSync(tmp, path)
}

/**
 * 获取进程内缓存的完整设置；首次调用会加载并迁移磁盘配置。
 *
 * @returns 已补齐默认值和迁移字段的应用设置。
 */
export function getSettings(): AppSettings {
  if (!cache) cache = load()
  return cache
}

/**
 * 合并局部设置并以原子写方式持久化。
 *
 * @param patch 需要覆盖的设置片段。
 * @returns 合并并落盘后的完整设置快照。
 */
export function updateSettings(patch: DeepPartial<AppSettings>): AppSettings {
  cache = deepMerge(getSettings(), patch) as AppSettings
  cache.player.volume = clampSetting(cache.player.volume, DEFAULT_SETTINGS.player.volume, 0, 1)
  persist(cache)
  return cache
}
