import type { ReplayGainInfo } from '../types/music'

export type ReplayGainMode = 'track' | 'album'

function finite(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

/** 过滤损坏或极端标签，避免元数据直接把音频节点推入无穷增益。 */
export function normalizeReplayGain(value: unknown): ReplayGainInfo | undefined {
  if (!value || typeof value !== 'object') return undefined
  const source = value as Record<string, unknown>
  const gain = (key: keyof ReplayGainInfo, min: number, max: number): number | undefined => {
    const number = finite(source[key])
    return number === undefined ? undefined : Math.max(min, Math.min(max, number))
  }
  const result: ReplayGainInfo = {
    trackDb: gain('trackDb', -60, 20),
    albumDb: gain('albumDb', -60, 20),
    trackPeak: gain('trackPeak', 0, 16),
    albumPeak: gain('albumPeak', 0, 16)
  }
  for (const key of Object.keys(result) as (keyof ReplayGainInfo)[]) {
    if (result[key] === undefined) delete result[key]
  }
  return Object.keys(result).length ? result : undefined
}

/** Resolve one playback gain. Positive gain is capped by the stored peak when available. */
export function resolveReplayGainDb(
  info: ReplayGainInfo | undefined,
  mode: ReplayGainMode,
  preampDb = 0,
  maxDb = 6
): number {
  const normalized = normalizeReplayGain(info)
  if (!normalized) return 0
  const album = mode === 'album'
  const gain = album ? (normalized.albumDb ?? normalized.trackDb) : (normalized.trackDb ?? normalized.albumDb)
  if (gain === undefined) return 0
  const peak = album ? (normalized.albumPeak ?? normalized.trackPeak) : (normalized.trackPeak ?? normalized.albumPeak)
  let resolved = gain + (Number.isFinite(preampDb) ? preampDb : 0)
  const upper = Math.max(0, Math.min(12, Number.isFinite(maxDb) ? maxDb : 6))
  if (resolved > 0 && peak !== undefined && peak > 0) {
    resolved = Math.min(resolved, -20 * Math.log10(peak))
  }
  const clamped = Math.max(-18, Math.min(upper, resolved))
  return Object.is(clamped, -0) ? 0 : clamped
}
