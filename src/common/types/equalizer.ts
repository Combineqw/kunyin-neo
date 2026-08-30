import { EQ_FREQUENCIES, EQ_GAIN_MAX, EQ_GAIN_MIN } from '../domain/equalizer'

export const EQUALIZER_PROFILE_FORMAT = 'kunyin-eq' as const
export const EQUALIZER_PROFILE_VERSION = 1 as const

export interface EqualizerBand {
  frequency: number
  gainDb: number
}

/** 与 Web Audio BiquadFilterNode 一一对应的动态滤镜描述。 */
export type EqualizerFilterType = 'peaking' | 'lowshelf' | 'highshelf'

export interface EqualizerFilter {
  type: EqualizerFilterType
  frequency: number
  gainDb: number
  q?: number
  enabled?: boolean
}

export interface EqualizerProfile {
  format: typeof EQUALIZER_PROFILE_FORMAT
  version: typeof EQUALIZER_PROFILE_VERSION
  name: string
  author: string
  description?: string
  preampDb?: number
  /** 兼容现有十段预设和旧版 .kyeq JSON。 */
  bands: EqualizerBand[]
  /** AutoEq / Equalizer APO 导入时保留的动态滤镜链；后续音频图阶段接入。 */
  filters?: EqualizerFilter[]
  source?: 'local' | 'official' | 'community' | 'imported'
  sourceUrl?: string
  checksum?: string
}

export type EqualizerProfileMeta = Omit<EqualizerProfile, 'bands' | 'filters'>

export interface EqualizerProfileImportResult {
  profile: EqualizerProfile
  filePath: string
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function finiteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function clampGain(value: number): number {
  return Math.max(EQ_GAIN_MIN, Math.min(EQ_GAIN_MAX, value))
}

function normalizeBand(value: unknown, index: number): EqualizerBand | null {
  if (finiteNumber(value)) {
    return { frequency: EQ_FREQUENCIES[index], gainDb: clampGain(value) }
  }
  if (!isRecord(value)) return null
  const rawFrequency = value.frequency
  if (rawFrequency !== undefined && rawFrequency !== EQ_FREQUENCIES[index]) return null
  const rawGain = value.gainDb ?? value.gain ?? value.value
  if (!finiteNumber(rawGain)) return null
  return { frequency: EQ_FREQUENCIES[index], gainDb: clampGain(rawGain) }
}

function normalizeText(value: unknown, fallback: string): string {
  return typeof value === 'string' && value.trim() ? value.trim().slice(0, 120) : fallback
}

function normalizeFilter(value: unknown): EqualizerFilter | null {
  if (!isRecord(value)) return null
  const type = value.type
  const frequency = value.frequency
  const gainDb = value.gainDb ?? value.gain
  const q = value.q
  if (
    (type !== 'peaking' && type !== 'lowshelf' && type !== 'highshelf') ||
    !finiteNumber(frequency) ||
    frequency <= 0 ||
    !finiteNumber(gainDb)
  ) {
    return null
  }
  if (q !== undefined && (!finiteNumber(q) || q <= 0)) return null
  return {
    type,
    frequency: Math.min(24000, frequency),
    // AutoEq 的动态滤镜可能超过十段滑杆的 +/-12 dB 范围，保留至 +/-24 dB。
    gainDb: Math.max(-24, Math.min(24, gainDb)),
    ...(q === undefined ? {} : { q: Math.min(100, q) }),
    ...(typeof value.enabled === 'boolean' ? { enabled: value.enabled } : {})
  }
}

export function normalizeEqualizerProfile(value: unknown): EqualizerProfile {
  if (!isRecord(value)) throw new Error('调音文件必须是 JSON 对象')
  if (value.format !== undefined && value.format !== EQUALIZER_PROFILE_FORMAT) {
    throw new Error('不支持的调音文件格式')
  }
  if (value.version !== undefined && value.version !== EQUALIZER_PROFILE_VERSION) {
    throw new Error('不支持的调音文件版本')
  }

  const rawBands = Array.isArray(value.bands)
    ? value.bands
    : Array.isArray(value.gains)
      ? value.gains
      : null
  if (!rawBands || rawBands.length !== EQ_FREQUENCIES.length) {
    throw new Error(`调音文件必须包含 ${EQ_FREQUENCIES.length} 个频段`)
  }

  const bands = rawBands.map((band, index) => normalizeBand(band, index))
  if (bands.some((band) => band === null)) throw new Error('调音文件包含无效的增益值')

  const profile: EqualizerProfile = {
    format: EQUALIZER_PROFILE_FORMAT,
    version: EQUALIZER_PROFILE_VERSION,
    name: normalizeText(value.name, '导入的调音'),
    author: normalizeText(value.author, '未知作者'),
    bands: bands as EqualizerBand[],
    source:
      value.source === 'local' ||
      value.source === 'official' ||
      value.source === 'community' ||
      value.source === 'imported'
        ? value.source
        : 'imported'
  }

  if (typeof value.description === 'string' && value.description.trim()) {
    profile.description = value.description.trim().slice(0, 500)
  }
  if (finiteNumber(value.preampDb)) profile.preampDb = Math.max(-24, Math.min(24, value.preampDb))
  if (Array.isArray(value.filters)) {
    const filters = value.filters.map(normalizeFilter)
    if (filters.some((filter) => filter === null)) throw new Error('调音文件包含无效的动态滤镜')
    profile.filters = filters as EqualizerFilter[]
  }
  if (typeof value.sourceUrl === 'string' && value.sourceUrl.trim()) {
    profile.sourceUrl = value.sourceUrl.trim().slice(0, 500)
  }
  if (typeof value.checksum === 'string' && value.checksum.trim()) {
    profile.checksum = value.checksum.trim().slice(0, 256)
  }
  return profile
}

export function validateEqualizerProfile(value: unknown): value is EqualizerProfile {
  try {
    normalizeEqualizerProfile(value)
    return true
  } catch {
    return false
  }
}

export function equalizerProfileFromGains(
  gains: readonly number[],
  meta?: Partial<EqualizerProfileMeta>
): EqualizerProfile {
  return normalizeEqualizerProfile({
    ...meta,
    bands: EQ_FREQUENCIES.map((frequency, index) => ({
      frequency,
      gainDb: gains[index] ?? 0
    }))
  })
}
