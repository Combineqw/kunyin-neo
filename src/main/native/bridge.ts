import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

type NativeBinding = {
  scanDirectory(dir: string): string
  parseTrack(path: string): string
  scanLyrics(dir: string): string
  readSettings(path: string): string
  writeSettings(path: string, settingsJson: string): boolean
  readAudioTags(path: string): string
  writeAudioTags(path: string, metadataJson: string): unknown
  playbackSnapshot?(): string
  playbackLoad?(trackId: string, durationMs: number): string
  playbackPlay?(): string
  playbackPause?(): string
  playbackSeek?(positionMs: number): string
  playbackTick?(elapsedMs: number): string
  playbackStop?(): string
  audioBackendCapabilities?(): string
  audioReplayGainLinear?(db: number): number
  audioReplayGainDb?(
    trackDb: number | null,
    albumDb: number | null,
    trackPeak: number | null,
    albumPeak: number | null,
    albumMode: boolean,
    preampDb: number,
    maxDb: number
  ): number
  themeMotionProfile?(
    themeId: string,
    reducedMotion: boolean,
    requestedFps?: number | null,
    displayRefreshHz?: number | null
  ): string
  nativeAudioStartFile?(path: string): string
  nativeAudioPlay?(): string
  nativeAudioPause?(): string
  nativeAudioSetVolume?(volume: number, muted: boolean): string
  nativeAudioSeek?(positionMs: number): string
  nativeAudioStop?(): string
  nativeAudioSnapshot?(): string
  audioDecryptQmc2Chunk?(ekey: string, fileOffset: number, chunk: Uint8Array): Uint8Array
  audioStreamCreate?(ekey: string): number
  audioStreamDecrypt?(id: number, fileOffset: number, chunk: Uint8Array): Uint8Array
  audioStreamClose?(id: number): boolean
  remoteStreamNegotiateFormat?(
    url: string,
    contentTypeHint?: string | null,
    upstreamContentType?: string | null,
    ekey?: string | null
  ): string
  remotePlaybackShadowCreate?(
    sessionId: string,
    url: string,
    contentTypeHint?: string | null,
    ekey?: string | null,
    durationMs?: number
  ): string
  remotePlaybackShadowUpdateResponse?(
    sessionId: string,
    statusCode: number,
    contentLength?: number | null,
    contentRange?: string | null,
    upstreamContentType?: string | null
  ): string | null
  remotePlaybackShadowReceiveChunk?(
    sessionId: string,
    offset: number,
    length: number
  ): string | null
  remotePlaybackShadowSeek?(sessionId: string, targetMs: number, durationMs: number): string | null
  remotePlaybackShadowSetPlayState?(sessionId: string, playing: boolean): string | null
  remotePlaybackShadowSnapshot?(sessionId: string): string | null
  remotePlaybackShadowClose?(sessionId: string): boolean
  audioDecryptQmc2File?(path: string, ekey: string): boolean
  syncNormalizeBaseUrl?(url: string): string
  syncValidateSession?(sessionJson: string): string
  providerParseQqTrack?(itemJson: string): string
  providerParseWyTrack?(itemJson: string): string
}

export type NativeAudioBackendCapabilities = {
  backend: string
  outputMode: 'htmlAudioFallback' | 'nativeShared' | 'nativeExclusive'
  canDecodeLocalFiles: boolean
  canOutputToDevice: boolean
  supportsExclusiveOutput: boolean
  boundedPcmQueue: boolean
  productionReady: boolean
  fallback: string
}

export type NativePlaybackSnapshot = {
  trackId: string | null
  status: 'idle' | 'paused' | 'playing' | 'ended'
  positionMs: number
  durationMs: number
}

export type NativeAudioSnapshot = {
  status: 'idle' | 'paused' | 'playing' | 'ended'
  positionMs: number
  durationMs: number | null
  format: { sampleRate: number; channels: number }
  deviceName: string | null
  queuedFrames: number
  droppedFrames: number
}

export type NativeNegotiatedFormat = {
  mimeType: string
  container: string
  encryption: 'none' | 'qmc2'
  normalizedUrl: string
  sourceHint: string
}

export type NativeRemotePlaybackShadowSnapshot = {
  sessionId: string
  format: NativeNegotiatedFormat
  status: 'idle' | 'buffering' | 'ready' | 'playing' | 'paused' | 'ended'
  totalBytes: number | null
  currentOffset: number
  bufferedBytes: number
  lowWatermarkBytes: number
  highWatermarkBytes: number
  watermarkStatus: 'buffering' | 'ready' | 'highWatermark' | 'ended'
  seekStatus: 'idle' | 'seeking' | 'buffering' | 'complete'
  seekGeneration: number
  seekTargetMs: number
  seekTargetByte: number | null
  positionMs: number
  durationMs: number
  underflowCount: number
}

export type NativeScanTrack = {
  title: string
  artist: string
  album: string
  duration: number
  path: string
  replayGain?: {
    trackDb?: number
    albumDb?: number
    trackPeak?: number
    albumPeak?: number
  }
}

export type NativeScanResult = {
  tracks: NativeScanTrack[]
  skippedNonAudio: number
  parseFailed: number
  walkErrors: number
}

export type NativeLyricLine = {
  start: number
  end: number
  text: string
}

export type NativeLyricFile = {
  path: string
  lines: NativeLyricLine[]
}

export type NativeAudioMetadata = {
  title?: string
  artist?: string
  album?: string
  trackNumber?: number
  lyrics?: string
  duration?: number
  replayGain?: NativeScanTrack['replayGain']
  pictureData?: number[]
  pictureMimeType?: string
}

const require = createRequire(import.meta.url)
let binding: NativeBinding | null | undefined

function nativeCandidates(): string[] {
  const file = 'aurora-native.win32-x64-msvc.node'
  return [
    join(process.resourcesPath, 'assets', file),
    join(process.cwd(), 'crates', 'aurora-native', file),
    join(__dirname, '..', '..', 'crates', 'aurora-native', file)
  ]
}

function loadBinding(): NativeBinding | null {
  if (binding !== undefined) return binding
  for (const candidate of nativeCandidates()) {
    if (!existsSync(candidate)) continue
    try {
      binding = require(candidate) as NativeBinding
      return binding
    } catch {
      // A stale or incompatible binary must not prevent the Node fallback.
    }
  }
  binding = null
  return null
}

export function nativeReadSettings(path: string): string | null {
  try {
    return loadBinding()?.readSettings(path) ?? null
  } catch {
    return null
  }
}

export function nativeWriteSettings(path: string, settings: object): boolean {
  try {
    const write = loadBinding()?.writeSettings
    if (typeof write !== 'function') return false
    const result = write(path, JSON.stringify(settings))
    return result === true
  } catch {
    return false
  }
}

export function nativeScanDirectory(path: string): NativeScanResult | null {
  try {
    const raw = loadBinding()?.scanDirectory(path)
    return raw ? (JSON.parse(raw) as NativeScanResult) : null
  } catch {
    return null
  }
}

export function nativeParseTrack(path: string): NativeScanTrack | null {
  try {
    const raw = loadBinding()?.parseTrack(path)
    return raw ? (JSON.parse(raw) as NativeScanTrack) : null
  } catch {
    return null
  }
}

export function nativeScanLyrics(path: string): NativeLyricFile[] | null {
  try {
    const raw = loadBinding()?.scanLyrics(path)
    return raw ? (JSON.parse(raw) as NativeLyricFile[]) : null
  } catch {
    return null
  }
}

export function nativeReadAudioTags(path: string): NativeAudioMetadata | null {
  try {
    const raw = loadBinding()?.readAudioTags(path)
    if (!raw) return null
    const value = JSON.parse(raw) as NativeAudioMetadata | null
    return value ?? null
  } catch {
    return null
  }
}

/** Best-effort native tag write; callers retain their TypeScript fallback. */
export function nativeWriteAudioTags(path: string, metadata: object): boolean {
  try {
    const write = loadBinding()?.writeAudioTags
    if (typeof write !== 'function') return false
    const metadataJson = JSON.stringify(metadata, (_key, value) => {
      if (value && value.type === 'Buffer' && Array.isArray(value.data)) return value.data
      return value
    })
    const result = write(path, metadataJson)
    return result !== false && result !== 'false'
  } catch {
    return false
  }
}

function nativePlaybackCall(
  method: keyof Pick<
    NativeBinding,
    | 'playbackSnapshot'
    | 'playbackLoad'
    | 'playbackPlay'
    | 'playbackPause'
    | 'playbackSeek'
    | 'playbackTick'
    | 'playbackStop'
  >,
  ...args: unknown[]
): NativePlaybackSnapshot | null {
  try {
    const call = loadBinding()?.[method] as ((...params: unknown[]) => string) | undefined
    if (typeof call !== 'function') return null
    const raw = call(...args)
    return raw ? (JSON.parse(raw) as NativePlaybackSnapshot) : null
  } catch {
    return null
  }
}

export function nativePlaybackSnapshot(): NativePlaybackSnapshot | null {
  return nativePlaybackCall('playbackSnapshot')
}

export function nativePlaybackLoad(
  trackId: string,
  durationMs: number
): NativePlaybackSnapshot | null {
  return nativePlaybackCall('playbackLoad', trackId, Math.max(0, Math.round(durationMs)))
}

export function nativePlaybackPlay(): NativePlaybackSnapshot | null {
  return nativePlaybackCall('playbackPlay')
}

export function nativePlaybackPause(): NativePlaybackSnapshot | null {
  return nativePlaybackCall('playbackPause')
}

export function nativePlaybackSeek(positionMs: number): NativePlaybackSnapshot | null {
  return nativePlaybackCall('playbackSeek', Math.max(0, Math.round(positionMs)))
}

export function nativePlaybackTick(elapsedMs: number): NativePlaybackSnapshot | null {
  return nativePlaybackCall('playbackTick', Math.max(0, Math.round(elapsedMs)))
}

export function nativePlaybackStop(): NativePlaybackSnapshot | null {
  return nativePlaybackCall('playbackStop')
}

export function nativeAudioBackendCapabilities(): NativeAudioBackendCapabilities | null {
  try {
    const call = loadBinding()?.audioBackendCapabilities
    if (typeof call !== 'function') return null
    const raw = call()
    return raw ? (JSON.parse(raw) as NativeAudioBackendCapabilities) : null
  } catch {
    return null
  }
}

/** Rust-owned ReplayGain dB boundary; renderer keeps its local fallback. */
export function nativeAudioReplayGainLinear(db: number): number | null {
  try {
    const call = loadBinding()?.audioReplayGainLinear
    if (typeof call !== 'function') return null
    const value = call(Number.isFinite(db) ? db : 0)
    return Number.isFinite(value) && value > 0 ? value : null
  } catch {
    return null
  }
}

/** Rust ReplayGain resolver for parameter-level shadow comparison. */
export function nativeAudioReplayGainDb(
  trackDb: number | null,
  albumDb: number | null,
  trackPeak: number | null,
  albumPeak: number | null,
  albumMode: boolean,
  preampDb: number,
  maxDb: number
): number | null {
  try {
    const call = loadBinding()?.audioReplayGainDb
    if (typeof call !== 'function') return null
    const value = call(
      Number.isFinite(trackDb) ? trackDb : null,
      Number.isFinite(albumDb) ? albumDb : null,
      Number.isFinite(trackPeak) ? trackPeak : null,
      Number.isFinite(albumPeak) ? albumPeak : null,
      albumMode === true,
      Number.isFinite(preampDb) ? preampDb : 0,
      Number.isFinite(maxDb) ? maxDb : 6
    )
    return Number.isFinite(value) ? value : null
  } catch {
    return null
  }
}

export type NativeThemeMotionProfile = {
  themeId: string
  enabled: boolean
  requestedFps: number | null
  displayRefreshHz: number | null
  targetFps: number | null
  frameIntervalMs: number
  blurPx: number
  opacity: number
  blobs: Array<{
    topPercent: number
    leftPercent: number
    sizePercent: number
    durationMs: number
    delayMs: number
  }>
}

export function nativeThemeMotionProfile(
  themeId: string,
  reducedMotion: boolean,
  requestedFps?: number | null,
  displayRefreshHz?: number | null
): NativeThemeMotionProfile | null {
  try {
    const call = loadBinding()?.themeMotionProfile
    if (typeof call !== 'function') return null
    const raw = call(themeId, reducedMotion, requestedFps ?? null, displayRefreshHz ?? null)
    return raw ? (JSON.parse(raw) as NativeThemeMotionProfile) : null
  } catch {
    return null
  }
}

function nativeAudioCall(
  method: keyof Pick<
    NativeBinding,
    | 'nativeAudioStartFile'
    | 'nativeAudioPlay'
    | 'nativeAudioPause'
    | 'nativeAudioSetVolume'
    | 'nativeAudioSeek'
    | 'nativeAudioStop'
    | 'nativeAudioSnapshot'
  >,
  ...args: unknown[]
): NativeAudioSnapshot | null {
  try {
    const call = loadBinding()?.[method] as ((...params: unknown[]) => string) | undefined
    if (typeof call !== 'function') return null
    const raw = call(...args)
    if (!raw) return null
    return JSON.parse(raw) as NativeAudioSnapshot | null
  } catch {
    return null
  }
}

export function nativeAudioStartFile(path: string): NativeAudioSnapshot | null {
  return nativeAudioCall('nativeAudioStartFile', path)
}

export function nativeAudioPlay(): NativeAudioSnapshot | null {
  return nativeAudioCall('nativeAudioPlay')
}

export function nativeAudioPause(): NativeAudioSnapshot | null {
  return nativeAudioCall('nativeAudioPause')
}

export function nativeAudioSetVolume(volume: number, muted: boolean): NativeAudioSnapshot | null {
  return nativeAudioCall('nativeAudioSetVolume', Math.max(0, Math.min(1, volume)), muted)
}

export function nativeAudioSeek(positionMs: number): NativeAudioSnapshot | null {
  return nativeAudioCall('nativeAudioSeek', Math.max(0, Math.round(positionMs)))
}

export function nativeAudioStop(): NativeAudioSnapshot | null {
  return nativeAudioCall('nativeAudioStop')
}

export function nativeAudioSnapshot(): NativeAudioSnapshot | null {
  return nativeAudioCall('nativeAudioSnapshot')
}

/** Decrypt one encrypted HTTP chunk through Rust when the optional binding has it. */
export function nativeAudioDecryptQmc2Chunk(
  ekey: string,
  fileOffset: number,
  chunk: Uint8Array
): Uint8Array | null {
  try {
    const call = loadBinding()?.audioDecryptQmc2Chunk
    if (typeof call !== 'function') return null
    const output = call(ekey, Math.max(0, Math.round(fileOffset)), chunk)
    return output instanceof Uint8Array ? output : null
  } catch {
    return null
  }
}

/** Optional stateful Rust QMC2 stream session for one Range response. */
export function nativeAudioStreamCreate(ekey: string): number | null {
  try {
    const call = loadBinding()?.audioStreamCreate
    if (typeof call !== 'function') return null
    const id = call(ekey)
    return Number.isSafeInteger(id) && id > 0 ? id : null
  } catch {
    return null
  }
}

export function nativeAudioStreamDecrypt(
  id: number,
  fileOffset: number,
  chunk: Uint8Array
): Uint8Array | null {
  try {
    const call = loadBinding()?.audioStreamDecrypt
    if (typeof call !== 'function') return null
    const output = call(id, Math.max(0, Math.round(fileOffset)), chunk)
    return output instanceof Uint8Array ? output : null
  } catch {
    return null
  }
}

export function nativeAudioStreamClose(id: number): boolean {
  try {
    const call = loadBinding()?.audioStreamClose
    return typeof call === 'function' ? call(id) === true : false
  } catch {
    return false
  }
}

/** Decrypt a downloaded QMC2 file in place with Rust's bounded buffer path. */
export function nativeAudioDecryptQmc2File(path: string, ekey: string): boolean {
  try {
    const call = loadBinding()?.audioDecryptQmc2File
    if (typeof call !== 'function') return false
    return call(path, ekey) === true
  } catch {
    return false
  }
}

/** Optional native URL normalization for the LX sync protocol. */
export function nativeSyncNormalizeBaseUrl(url: string): string | null {
  try {
    const call = loadBinding()?.syncNormalizeBaseUrl
    if (typeof call !== 'function') return null
    return call(url)
  } catch {
    return null
  }
}

/** Optional native validation of the persisted LX sync session. */
export function nativeSyncValidateSession<T extends object>(sessionJson: string): T | null {
  try {
    const call = loadBinding()?.syncValidateSession
    if (typeof call !== 'function') return null
    const raw = call(sessionJson)
    return raw ? (JSON.parse(raw) as T | null) : null
  } catch {
    return null
  }
}

/** Optional Rust QQ provider item mapping. */
export function nativeParseQqTrack<T extends object>(item: unknown): T | null {
  try {
    const call = loadBinding()?.providerParseQqTrack
    if (typeof call !== 'function') return null
    const raw = call(JSON.stringify(item))
    if (!raw) return null
    return JSON.parse(raw) as T | null
  } catch {
    return null
  }
}

/** Optional Rust Netease provider item mapping. */
export function nativeParseWyTrack<T extends object>(item: unknown): T | null {
  try {
    const call = loadBinding()?.providerParseWyTrack
    if (typeof call !== 'function') return null
    const raw = call(JSON.stringify(item))
    if (!raw) return null
    return JSON.parse(raw) as T | null
  } catch {
    return null
  }
}

/** Optional native remote stream format negotiation. */
export function nativeRemoteStreamNegotiateFormat(
  url: string,
  contentTypeHint?: string | null,
  upstreamContentType?: string | null,
  ekey?: string | null
): NativeNegotiatedFormat | null {
  try {
    const call = loadBinding()?.remoteStreamNegotiateFormat
    if (typeof call !== 'function') return null
    const raw = call(url, contentTypeHint ?? null, upstreamContentType ?? null, ekey ?? null)
    return raw ? (JSON.parse(raw) as NativeNegotiatedFormat) : null
  } catch {
    return null
  }
}

/** Optional native shadow session creation for remote stream tracking. */
export function nativeRemotePlaybackShadowCreate(
  sessionId: string,
  url: string,
  contentTypeHint?: string | null,
  ekey?: string | null,
  durationMs = 0
): NativeRemotePlaybackShadowSnapshot | null {
  try {
    const call = loadBinding()?.remotePlaybackShadowCreate
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      url,
      contentTypeHint ?? null,
      ekey ?? null,
      Math.max(0, Math.round(durationMs))
    )
    return raw ? (JSON.parse(raw) as NativeRemotePlaybackShadowSnapshot) : null
  } catch {
    return null
  }
}

/** Optional native shadow session update with upstream HTTP response metadata. */
export function nativeRemotePlaybackShadowUpdateResponse(
  sessionId: string,
  statusCode: number,
  contentLength?: number | null,
  contentRange?: string | null,
  upstreamContentType?: string | null
): NativeRemotePlaybackShadowSnapshot | null {
  try {
    const call = loadBinding()?.remotePlaybackShadowUpdateResponse
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      statusCode,
      contentLength == null ? null : Math.max(0, Math.round(contentLength)),
      contentRange ?? null,
      upstreamContentType ?? null
    )
    return raw ? (JSON.parse(raw) as NativeRemotePlaybackShadowSnapshot) : null
  } catch {
    return null
  }
}

/** Optional native shadow session update with received chunk bytes. */
export function nativeRemotePlaybackShadowReceiveChunk(
  sessionId: string,
  offset: number,
  length: number
): NativeRemotePlaybackShadowSnapshot | null {
  try {
    const call = loadBinding()?.remotePlaybackShadowReceiveChunk
    if (typeof call !== 'function') return null
    const raw = call(sessionId, Math.max(0, Math.round(offset)), Math.max(0, Math.round(length)))
    return raw ? (JSON.parse(raw) as NativeRemotePlaybackShadowSnapshot) : null
  } catch {
    return null
  }
}

/** Optional native shadow session seek recording. */
export function nativeRemotePlaybackShadowSeek(
  sessionId: string,
  targetMs: number,
  durationMs = 0
): NativeRemotePlaybackShadowSnapshot | null {
  try {
    const call = loadBinding()?.remotePlaybackShadowSeek
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      Math.max(0, Math.round(targetMs)),
      Math.max(0, Math.round(durationMs))
    )
    return raw ? (JSON.parse(raw) as NativeRemotePlaybackShadowSnapshot) : null
  } catch {
    return null
  }
}

/** Optional native shadow session play state recording. */
export function nativeRemotePlaybackShadowSetPlayState(
  sessionId: string,
  playing: boolean
): NativeRemotePlaybackShadowSnapshot | null {
  try {
    const call = loadBinding()?.remotePlaybackShadowSetPlayState
    if (typeof call !== 'function') return null
    const raw = call(sessionId, !!playing)
    return raw ? (JSON.parse(raw) as NativeRemotePlaybackShadowSnapshot) : null
  } catch {
    return null
  }
}

/** Optional native shadow session snapshot query. */
export function nativeRemotePlaybackShadowSnapshot(
  sessionId: string
): NativeRemotePlaybackShadowSnapshot | null {
  try {
    const call = loadBinding()?.remotePlaybackShadowSnapshot
    if (typeof call !== 'function') return null
    const raw = call(sessionId)
    return raw ? (JSON.parse(raw) as NativeRemotePlaybackShadowSnapshot) : null
  } catch {
    return null
  }
}

/** Optional native shadow session close. */
export function nativeRemotePlaybackShadowClose(sessionId: string): boolean {
  try {
    const call = loadBinding()?.remotePlaybackShadowClose
    return typeof call === 'function' ? call(sessionId) === true : false
  } catch {
    return false
  }
}
