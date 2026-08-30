/**
 * 外部 EQ 文本格式的纯函数解析与序列化。
 * 本模块不持有音频上下文；输出的预设描述由渲染层音频图按需映射为 BiquadFilterNode。
 */
import { EQ_FREQUENCIES } from './equalizer'
import {
  EQUALIZER_PROFILE_FORMAT,
  EQUALIZER_PROFILE_VERSION,
  normalizeEqualizerProfile,
  type EqualizerFilter,
  type EqualizerProfile
} from '../types/equalizer'

const FILTER_TYPE_MAP: Record<string, EqualizerFilter['type']> = {
  PK: 'peaking',
  LSC: 'lowshelf',
  HSC: 'highshelf'
}

function parseNumber(value: string, line: number, label: string): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) throw new Error(`第 ${line} 行的 ${label} 不是有效数值`)
  return parsed
}

function profileName(sourceName?: string): string {
  const name = sourceName?.trim().replace(/\.[^.]+$/, '')
  return name || '导入的调音'
}

/**
 * 解析 Equalizer APO 与 AutoEq 的基础文本格式。
 * @param text 外部调音文本。
 * @param sourceName 可选的导入文件名，用于生成预设显示名。
 * @returns 规范化的动态滤镜预设。
 * @throws 遇到不支持的 Filter 语法时附带行号抛出错误。
 */
export function parseEqualizerApoText(text: string, sourceName?: string): EqualizerProfile {
  const filters: EqualizerFilter[] = []
  let preampDb = 0
  let sawConfig = false
  const lines = text.replace(/^\uFEFF/, '').split(/\r?\n/)

  for (let index = 0; index < lines.length; index += 1) {
    const lineNumber = index + 1
    const raw = lines[index].trim()
    if (!raw || raw.startsWith('#') || raw.startsWith(';') || /^Device:/i.test(raw)) continue

    const preamp = raw.match(/^Preamp:\s*([+-]?(?:\d+(?:\.\d+)?|\.\d+))\s*dB\s*$/i)
    if (preamp) {
      preampDb = parseNumber(preamp[1], lineNumber, 'Preamp')
      sawConfig = true
      continue
    }

    const filter = raw.match(
      /^Filter\s+\d+:\s*(ON|OFF)\s+(PK|LSC|HSC)\s+Fc\s+([+-]?(?:\d+(?:\.\d+)?|\.\d+))\s*Hz\s+Gain\s+([+-]?(?:\d+(?:\.\d+)?|\.\d+))\s*dB(?:\s+Q\s+([+-]?(?:\d+(?:\.\d+)?|\.\d+)))?\s*$/i
    )
    if (filter) {
      const token = filter[2].toUpperCase()
      const type = FILTER_TYPE_MAP[token]
      if (!type) throw new Error(`第 ${lineNumber} 行的滤镜类型 ${token} 不受支持`)
      const frequency = parseNumber(filter[3], lineNumber, 'Fc')
      const gainDb = parseNumber(filter[4], lineNumber, 'Gain')
      const q = filter[5] ? parseNumber(filter[5], lineNumber, 'Q') : undefined
      if (frequency <= 0) throw new Error(`第 ${lineNumber} 行的 Fc 必须大于 0`)
      if (q !== undefined && q <= 0) throw new Error(`第 ${lineNumber} 行的 Q 必须大于 0`)
      filters.push({ type, frequency, gainDb, ...(q === undefined ? {} : { q }), enabled: filter[1].toUpperCase() === 'ON' })
      sawConfig = true
      continue
    }

    if (/^Filter\s+\d+:/i.test(raw) || /^Preamp:/i.test(raw)) {
      throw new Error(`第 ${lineNumber} 行不是支持的 Equalizer APO / AutoEq 配置`)
    }
  }

  if (!sawConfig) throw new Error('未找到 Preamp 或 Filter 配置行')
  return normalizeEqualizerProfile({
    format: EQUALIZER_PROFILE_FORMAT,
    version: EQUALIZER_PROFILE_VERSION,
    name: profileName(sourceName),
    author: 'AutoEq / Equalizer APO',
    source: 'imported',
    preampDb,
    filters,
    bands: EQ_FREQUENCIES.map((frequency) => ({ frequency, gainDb: 0 }))
  })
}

/**
 * 将每行一个 dB 数值或逗号分隔的固定十段增益文本转换为规范预设。
 * @param text 原始十段增益文本。
 * @param sourceName 可选的导入文件名，用于生成预设显示名。
 * @returns 可直接持久化的标准 EqualizerProfile。
 */
/**
 * 解析 AutoEq/Equalizer APO 的 GraphicEQ 频响曲线。
 * 应用播放链固定为十段图形均衡器，因此按对数频率在原曲线上插值，映射到固定频段。
 */
export function parseGraphicEqText(text: string, sourceName?: string): EqualizerProfile {
  const match = text.replace(/^\uFEFF/, '').match(/(?:^|\n)\s*GraphicEQ:\s*([\s\S]*)$/i)
  if (!match) throw new Error('未找到 GraphicEQ 配置行')

  const points = Array.from(
    match[1].matchAll(
      /([+-]?(?:\d+(?:\.\d+)?|\.\d+))\s+([+-]?(?:\d+(?:\.\d+)?|\.\d+))/g
    )
  ).map((item, index) => ({
    frequency: parseNumber(item[1], index + 1, '频率'),
    gainDb: parseNumber(item[2], index + 1, '增益')
  }))
  if (points.length < 2 || points.some((point) => point.frequency <= 0)) {
    throw new Error('GraphicEQ 至少需要两个频率-增益点，且频率必须大于 0')
  }
  points.sort((left, right) => left.frequency - right.frequency)
  for (let index = 1; index < points.length; index += 1) {
    if (points[index].frequency <= points[index - 1].frequency) {
      throw new Error('GraphicEQ 频率必须严格递增')
    }
  }

  const gainAt = (frequency: number): number => {
    if (frequency <= points[0].frequency) return points[0].gainDb
    if (frequency >= points[points.length - 1].frequency) return points[points.length - 1].gainDb
    const rightIndex = points.findIndex((point) => point.frequency >= frequency)
    const left = points[rightIndex - 1]
    const right = points[rightIndex]
    const progress =
      (Math.log(frequency) - Math.log(left.frequency)) /
      (Math.log(right.frequency) - Math.log(left.frequency))
    return left.gainDb + (right.gainDb - left.gainDb) * progress
  }

  return normalizeEqualizerProfile({
    format: EQUALIZER_PROFILE_FORMAT,
    version: EQUALIZER_PROFILE_VERSION,
    name: profileName(sourceName),
    author: 'AutoEq / Equalizer APO',
    description: `GraphicEQ 曲线已按对数频率映射到 ${EQ_FREQUENCIES.length} 段图形均衡器`,
    source: 'imported',
    bands: EQ_FREQUENCIES.map((frequency) => ({ frequency, gainDb: gainAt(frequency) }))
  })
}

export function parseTenBandGainList(text: string, sourceName?: string): EqualizerProfile {
  const values = text
    .replace(/^\uFEFF/, '')
    .split(/[\s,;]+/)
    .filter(Boolean)
  if (values.length !== EQ_FREQUENCIES.length) {
    throw new Error(`十段增益列表必须包含 ${EQ_FREQUENCIES.length} 个数值，当前为 ${values.length} 个`)
  }
  const gains = values.map((value, index) => parseNumber(value.replace(/dB$/i, ''), index + 1, '增益'))
  return normalizeEqualizerProfile({
    format: EQUALIZER_PROFILE_FORMAT,
    version: EQUALIZER_PROFILE_VERSION,
    name: profileName(sourceName),
    author: '导入',
    source: 'imported',
    bands: EQ_FREQUENCIES.map((frequency, index) => ({ frequency, gainDb: gains[index] }))
  })
}

/**
 * 按文本特征选择 AutoEq/APO 或十段增益解析器。
 * @param text 外部调音文本。
 * @param sourceName 可选的导入文件名。
 * @returns 规范化后的 EqualizerProfile。
 */
export function parseExternalEqualizerText(text: string, sourceName?: string): EqualizerProfile {
  if (/(?:^|\n)\s*GraphicEQ:/i.test(text)) return parseGraphicEqText(text, sourceName)
  return /(?:^|\n)\s*(?:Preamp:|Filter\s+\d+:)/i.test(text)
    ? parseEqualizerApoText(text, sourceName)
    : parseTenBandGainList(text, sourceName)
}

/**
 * 将当前预设导出为兼容 Equalizer APO 的文本。
 * @param profile 待导出的 EQ 预设。
 * @returns 可保存为文本文件的 APO 配置内容。
 */
export function serializeEqualizerApoText(profile: EqualizerProfile): string {
  const normalized = normalizeEqualizerProfile(profile)
  const lines = [`# ${normalized.name}`, `Preamp: ${(normalized.preampDb ?? 0).toFixed(2)} dB`]
  for (const [index, filter] of (normalized.filters ?? []).entries()) {
    const type = filter.type === 'peaking' ? 'PK' : filter.type === 'lowshelf' ? 'LSC' : 'HSC'
    const enabled = filter.enabled === false ? 'OFF' : 'ON'
    const q = filter.q == null ? '' : ` Q ${filter.q}`
    lines.push(`Filter ${index + 1}: ${enabled} ${type} Fc ${filter.frequency} Hz Gain ${filter.gainDb} dB${q}`)
  }
  return `${lines.join('\n')}\n`
}
