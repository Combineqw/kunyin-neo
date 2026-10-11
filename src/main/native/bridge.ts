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
  remoteStreamPlanRange?(
    sessionId: string,
    currentOffset?: number | null,
    chunkSize?: number | null
  ): string | null
  remoteStreamGetBufferedRanges?(sessionId: string): string | null
  remoteStreamIsOffsetBuffered?(sessionId: string, offset: number): boolean
  remoteStreamCalculateSeekRange?(
    sessionId: string,
    targetMs: number,
    durationMs: number,
    alignBytes?: number | null
  ): string | null
  streamCacheCreate?(
    cacheId: string,
    maxMemoryBytes?: number | null,
    tempDir?: string | null
  ): string
  streamCacheWriteChunk?(cacheId: string, offset: number, chunk: Uint8Array): number
  streamCacheRead?(cacheId: string, offset: number, maxLen: number): Uint8Array | null
  streamCacheStats?(cacheId: string): string | null
  streamCacheSetTotalLength?(cacheId: string, totalLength: number): boolean
  streamCacheClose?(cacheId: string): boolean
  streamDecoderCreate?(
    decoderId: string,
    cacheId: string,
    containerHint?: string | null,
    mimeHint?: string | null
  ): string
  streamDecoderProbe?(decoderId: string): string | null
  streamDecoderStep?(decoderId: string): string
  streamDecoderSeek?(decoderId: string, targetMs: number): string
  streamDecoderSnapshot?(decoderId: string): string | null
  streamDecoderClose?(decoderId: string): boolean
  dspPipelineCreate?(
    pipelineId: string,
    sampleRate: number,
    channels: number,
    configJson?: string | null
  ): string
  dspPipelineConfigure?(pipelineId: string, configJson: string): string
  dspPipelineProcess?(pipelineId: string, chunk: Uint8Array): Uint8Array | null
  dspPipelineReset?(pipelineId: string): boolean
  dspPipelineSnapshot?(pipelineId: string): string | null
  dspPipelineClose?(pipelineId: string): boolean
  audioSinkCreate?(
    sinkId: string,
    cacheId: string,
    containerHint?: string | null,
    mimeHint?: string | null,
    dspConfigJson?: string | null,
    forceVirtual?: boolean | null
  ): string
  audioSinkPlay?(sinkId: string): boolean
  audioSinkPause?(sinkId: string): boolean
  audioSinkStop?(sinkId: string): boolean
  audioSinkSeek?(sinkId: string, targetMs: number): boolean
  audioSinkSetVolume?(sinkId: string, volume: number, muted: boolean): boolean
  audioSinkConfigureDsp?(sinkId: string, dspConfigJson: string): string
  audioSinkSnapshot?(sinkId: string): string | null
  audioSinkClose?(sinkId: string): boolean
  audioDevicesEnumerate?(): string
  audioSinkSwitchDevice?(
    sinkId: string,
    deviceName?: string | null,
    exclusive?: boolean | null
  ): boolean
  audioDecryptQmc2File?(path: string, ekey: string): boolean
  syncNormalizeBaseUrl?(url: string): string
  syncValidateSession?(sessionJson: string): string
  providerParseQqTrack?(itemJson: string): string
  providerParseWyTrack?(itemJson: string): string
  providerParseKgTrack?(itemJson: string): string
  providerParseKwTrack?(itemJson: string): string
  providerKgSign?(paramsJson: string, body: string): string
  providerExtractBackendMusicId?(
    platform: string,
    id?: string | null,
    mid?: string | null,
    hash?: string | null
  ): string
  providerSelectOptimalQuality?(availableQualities: string[], preferredQuality: string): string
  providerParseMediaInfoResponse?(
    source: string,
    qualityId: string,
    responseJson: string,
    nowMs: number
  ): string
  providerParsePlaylistEntry?(source: string, entryJson: string): string
  providerParseAlbumEntry?(source: string, entryJson: string): string
  providerParseArtistEntry?(source: string, entryJson: string): string
  lyricsParseLrc?(content: string): string
  lyricsFindActiveLine?(linesJson: string, timeMs: number): string | null
  desktopLyricComputeAppearance?(
    bgOpacity: number,
    textOpacity: number,
    fontWeight: number,
    shadowOpacity: number,
    strokeWidth: number
  ): string
  miniPlayerComputeAppearance?(opacityPercent: number): string
  networkBuildHeaders?(
    provider: string,
    targetUrl: string,
    rangeStart?: number | null,
    rangeEnd?: number | null
  ): string
  networkPlanStreamRange?(
    startByte: number,
    totalBytes: number,
    chunkSize: number
  ): string
  networkValidateProxy?(enable: boolean, host: string, port: number): string
  lyricsParseSyllables?(lrcText: string): string
  lyricsComputeSyllableProgress?(lineJson: string, timeMs: number): string
  lyricsRenderDesktopFrame?(
    mainLine: string,
    subLine: string | null | undefined,
    progress: number,
    activeColor: string,
    normalColor: string,
    bgOpacity: number,
    textOpacity: number,
    fontSize: number,
    isPlaying: boolean
  ): string
  matcherNormalizeMusicText?(text: string): string
  matcherArtistsMatch?(left: string, right: string): boolean
  matcherSelectEnrichmentCandidate?(
    tagsJson: string,
    durationMs: number,
    candidatesJson: string
  ): string | null
  matcherScoreMusicMatch?(
    tagTitle: string,
    tagArtist: string,
    tagDurationMs: number,
    candidateTitle: string,
    candidateArtist: string,
    candidateDurationMs: number
  ): number
  matcherRankCandidates?(
    tagsJson: string,
    durationMs: number,
    candidatesJson: string
  ): string
  downloadSniffAudioExtension?(filePath: string): string | null
  downloadParseFlacQualityTag?(filePath: string): string | null
  downloadNormalizeTitle?(title: string): string
  downloadSelectAlternativeCandidate?(
    targetTitle: string,
    candidatesJson: string
  ): string | null
}

export type NativeAudioDeviceInfo = {
  name: string
  isDefault: boolean
  minSampleRate: number
  maxSampleRate: number
  channels: number
  supportsExclusive: boolean
}

export type NativeSinkStatus = 'idle' | 'buffering' | 'playing' | 'paused' | 'ended' | 'error'

export type NativeSinkSnapshot = {
  sinkId: string
  status: NativeSinkStatus
  positionMs: number
  durationMs: number | null
  sourceFormat: { sampleRate: number; channels: number } | null
  outputFormat: { sampleRate: number; channels: number }
  queuedFrames: number
  droppedFrames: number
  deviceName: string | null
  dsp: NativeDspPipelineSnapshot
  underflowCount: number
  isVirtual: boolean
  isExclusive: boolean
}

export type NativeDspPipelineConfig = {
  eqEnabled: boolean
  eqGainsDb: number[]
  replayGainDb?: number | null
  volume: number
  muted: boolean
  limiterEnabled: boolean
}

export type NativeDspPipelineSnapshot = {
  format: {
    sampleRate: number
    channels: number
  }
  config: NativeDspPipelineConfig
  eqActive: boolean
  currentGainLinear: number
  replayGainLinear: number
  peakLevel: number
}

export type NativeCacheStats = {
  cacheId: string
  totalCachedBytes: number
  memoryBytes: number
  fileBytes: number
  readCursor: number
  contiguousForwardBytes: number
  bufferedRangeCount: number
  isSpilledToFile: boolean
}

export interface NativeStreamDecoderSnapshot {
  streamId: string
  state: 'awaitingHeader' | 'ready' | 'decoding' | 'underflow' | 'endOfStream' | 'failed'
  format: {
    sampleRate: number
    channels: number
  } | null
  codec: string | null
  durationMs: number | null
  decodedFrames: number
  positionMs: number
  bufferedForwardBytes: number
  totalCachedBytes: number
}

export interface NativeStreamDecodeStepResult {
  status: 'chunk' | 'needMoreData' | 'endOfStream'
  frames: number
  endOfStream: boolean
  underflow: boolean
  positionMs: number
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

export type NativeByteRange = {
  start: number
  end: number
}

export type NativeRangePlan = {
  rangeStart: number
  rangeEnd: number
  priority: 'critical' | 'high' | 'normal' | 'idle'
  isCached: boolean
  chunkSize: number
  httpRangeHeader: string
}

export type NativeSeekRangeCalculation = {
  targetMs: number
  estimatedByte: number
  alignedRangeStart: number
  plannedRangeEnd: number
  httpRangeHeader: string
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
  bufferedRangeCount: number
  totalBufferedBytes: number
  isCurrentCached: boolean
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

/** Optional Rust KuGou provider item mapping. */
export function nativeParseKgTrack<T extends object>(item: unknown): T | null {
  try {
    const call = loadBinding()?.providerParseKgTrack
    if (typeof call !== 'function') return null
    const raw = call(JSON.stringify(item))
    if (!raw) return null
    return JSON.parse(raw) as T | null
  } catch {
    return null
  }
}

/** Optional Rust KuWo provider item mapping. */
export function nativeParseKwTrack<T extends object>(item: unknown): T | null {
  try {
    const call = loadBinding()?.providerParseKwTrack
    if (typeof call !== 'function') return null
    const raw = call(JSON.stringify(item))
    if (!raw) return null
    return JSON.parse(raw) as T | null
  } catch {
    return null
  }
}

/** Optional Rust KuGou MD5 request signature calculation. */
export function nativeKgSign(params: Record<string, string>, body = ''): string | null {
  try {
    const call = loadBinding()?.providerKgSign
    if (typeof call !== 'function') return null
    return call(JSON.stringify(params), body)
  } catch {
    return null
  }
}

/** Optional Rust platform music ID extraction. */
export function nativeExtractBackendMusicId(
  platform: string,
  id?: string | number | null,
  mid?: string | null,
  hash?: string | null
): string {
  try {
    const call = loadBinding()?.providerExtractBackendMusicId
    if (typeof call === 'function') {
      const idStr = id != null ? String(id) : null
      return call(platform, idStr, mid ?? null, hash ?? null)
    }
  } catch {
    // fallback
  }
  switch (platform) {
    case 'qq':
      return mid ?? ''
    case 'joox':
      return mid ?? (id != null ? String(id) : '')
    case 'kg':
      return (hash ?? '').toLowerCase()
    default:
      return id != null ? String(id) : ''
  }
}

/** Optional Rust audio quality resolution with graceful fallback. */
export function nativeSelectOptimalQuality(
  availableQualities: string[],
  preferredQuality: string
): string {
  try {
    const call = loadBinding()?.providerSelectOptimalQuality
    if (typeof call === 'function') {
      return call(availableQualities, preferredQuality)
    }
  } catch {
    // fallback
  }
  if (!availableQualities.length) return '128k'
  if (availableQualities.includes(preferredQuality)) return preferredQuality
  return availableQualities[0]
}

/** Optional Rust getUrl response parser. */
export function nativeParseMediaInfoResponse<T extends object>(
  source: string,
  qualityId: string,
  response: unknown,
  nowMs = Date.now()
): T | null {
  try {
    const call = loadBinding()?.providerParseMediaInfoResponse
    if (typeof call !== 'function') return null
    const raw = call(source, qualityId, JSON.stringify(response), nowMs)
    return raw ? (JSON.parse(raw) as T) : null
  } catch {
    return null
  }
}

export interface NativePlaylistEntry {
  source: string
  id: string
  name: string
  cover: string
  creator: string
  description: string
  total: number
}

export interface NativeAlbumEntry {
  source: string
  id: string
  name: string
  cover: string
  artist: string
  publishTime?: string
  total: number
}

export interface NativeArtistEntry {
  source: string
  id: string
  name: string
  avatar: string
  albumSize: number
  musicSize: number
}

export function nativeParsePlaylistEntry(source: string, entry: unknown): NativePlaylistEntry | null {
  try {
    const call = loadBinding()?.providerParsePlaylistEntry
    if (typeof call !== 'function') return null
    const raw = call(source, JSON.stringify(entry))
    return raw ? (JSON.parse(raw) as NativePlaylistEntry) : null
  } catch {
    return null
  }
}

export function nativeParseAlbumEntry(source: string, entry: unknown): NativeAlbumEntry | null {
  try {
    const call = loadBinding()?.providerParseAlbumEntry
    if (typeof call !== 'function') return null
    const raw = call(source, JSON.stringify(entry))
    return raw ? (JSON.parse(raw) as NativeAlbumEntry) : null
  } catch {
    return null
  }
}

export function nativeParseArtistEntry(source: string, entry: unknown): NativeArtistEntry | null {
  try {
    const call = loadBinding()?.providerParseArtistEntry
    if (typeof call !== 'function') return null
    const raw = call(source, JSON.stringify(entry))
    return raw ? (JSON.parse(raw) as NativeArtistEntry) : null
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

/** Optional native remote stream chunk Range planning. */
export function nativeRemoteStreamPlanRange(
  sessionId: string,
  currentOffset?: number | null,
  chunkSize?: number | null
): NativeRangePlan | null {
  try {
    const call = loadBinding()?.remoteStreamPlanRange
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      currentOffset == null ? null : Math.max(0, Math.round(currentOffset)),
      chunkSize == null ? null : Math.max(0, Math.round(chunkSize))
    )
    return raw ? (JSON.parse(raw) as NativeRangePlan) : null
  } catch {
    return null
  }
}

/** Optional query for list of buffered byte ranges in the shadow session. */
export function nativeRemoteStreamGetBufferedRanges(sessionId: string): NativeByteRange[] | null {
  try {
    const call = loadBinding()?.remoteStreamGetBufferedRanges
    if (typeof call !== 'function') return null
    const raw = call(sessionId)
    return raw ? (JSON.parse(raw) as NativeByteRange[]) : null
  } catch {
    return null
  }
}

/** Optional fast check whether a specific offset is already buffered. */
export function nativeRemoteStreamIsOffsetBuffered(sessionId: string, offset: number): boolean {
  try {
    const call = loadBinding()?.remoteStreamIsOffsetBuffered
    if (typeof call !== 'function') return false
    return call(sessionId, Math.max(0, Math.round(offset))) === true
  } catch {
    return false
  }
}

/** Optional calculation of aligned byte range for a target seek position. */
export function nativeRemoteStreamCalculateSeekRange(
  sessionId: string,
  targetMs: number,
  durationMs = 0,
  alignBytes?: number | null
): NativeSeekRangeCalculation | null {
  try {
    const call = loadBinding()?.remoteStreamCalculateSeekRange
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      Math.max(0, Math.round(targetMs)),
      Math.max(0, Math.round(durationMs)),
      alignBytes == null ? null : Math.max(0, Math.round(alignBytes))
    )
    return raw ? (JSON.parse(raw) as NativeSeekRangeCalculation) : null
  } catch {
    return null
  }
}

/** Optional initialization of native bounded stream cache. */
export function nativeStreamCacheCreate(
  cacheId: string,
  maxMemoryBytes?: number | null,
  tempDir?: string | null
): NativeCacheStats | null {
  try {
    const call = loadBinding()?.streamCacheCreate
    if (typeof call !== 'function') return null
    const raw = call(
      cacheId,
      maxMemoryBytes == null ? null : Math.max(0, Math.round(maxMemoryBytes)),
      tempDir ?? null
    )
    return raw ? (JSON.parse(raw) as NativeCacheStats) : null
  } catch {
    return null
  }
}

/** Optional chunk write into native stream cache. */
export function nativeStreamCacheWriteChunk(
  cacheId: string,
  offset: number,
  chunk: Uint8Array
): number {
  try {
    const call = loadBinding()?.streamCacheWriteChunk
    if (typeof call !== 'function') return 0
    const written = call(cacheId, Math.max(0, Math.round(offset)), chunk)
    return Number.isSafeInteger(written) && written > 0 ? written : 0
  } catch {
    return 0
  }
}

/** Optional slice read from native stream cache. */
export function nativeStreamCacheRead(
  cacheId: string,
  offset: number,
  maxLen: number
): Uint8Array | null {
  try {
    const call = loadBinding()?.streamCacheRead
    if (typeof call !== 'function') return null
    const output = call(cacheId, Math.max(0, Math.round(offset)), Math.max(0, Math.round(maxLen)))
    return output instanceof Uint8Array ? output : null
  } catch {
    return null
  }
}

/** Optional stats query for native stream cache. */
export function nativeStreamCacheStats(cacheId: string): NativeCacheStats | null {
  try {
    const call = loadBinding()?.streamCacheStats
    if (typeof call !== 'function') return null
    const raw = call(cacheId)
    return raw ? (JSON.parse(raw) as NativeCacheStats) : null
  } catch {
    return null
  }
}

/** Optional total length declaration for native stream cache. */
export function nativeStreamCacheSetTotalLength(cacheId: string, totalLength: number): boolean {
  try {
    const call = loadBinding()?.streamCacheSetTotalLength
    return typeof call === 'function'
      ? call(cacheId, Math.max(0, Math.round(totalLength))) === true
      : false
  } catch {
    return false
  }
}

/** Optional close and cleanup of native stream cache. */
export function nativeStreamCacheClose(cacheId: string): boolean {
  try {
    const call = loadBinding()?.streamCacheClose
    return typeof call === 'function' ? call(cacheId) === true : false
  } catch {
    return false
  }
}

/** Optional creation of native stream decoder bound to a stream cache. */
export function nativeStreamDecoderCreate(
  decoderId: string,
  cacheId: string,
  containerHint?: string | null,
  mimeHint?: string | null
): NativeStreamDecoderSnapshot | null {
  try {
    const call = loadBinding()?.streamDecoderCreate
    if (typeof call !== 'function') return null
    const raw = call(decoderId, cacheId, containerHint ?? null, mimeHint ?? null)
    return raw ? (JSON.parse(raw) as NativeStreamDecoderSnapshot) : null
  } catch {
    return null
  }
}

/** Optional format probe for stream decoder. */
export function nativeStreamDecoderProbe(decoderId: string): {
  format: { sampleRate: number; channels: number }
  codec: string
  durationFrames?: number
} | null {
  try {
    const call = loadBinding()?.streamDecoderProbe
    if (typeof call !== 'function') return null
    const raw = call(decoderId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

/** Optional single-step packet decoding. */
export function nativeStreamDecoderStep(decoderId: string): NativeStreamDecodeStepResult | null {
  try {
    const call = loadBinding()?.streamDecoderStep
    if (typeof call !== 'function') return null
    const raw = call(decoderId)
    return raw ? (JSON.parse(raw) as NativeStreamDecodeStepResult) : null
  } catch {
    return null
  }
}

/** Optional seek of native stream decoder to target timestamp in milliseconds. */
export function nativeStreamDecoderSeek(
  decoderId: string,
  targetMs: number
): NativeStreamDecoderSnapshot | null {
  try {
    const call = loadBinding()?.streamDecoderSeek
    if (typeof call !== 'function') return null
    const raw = call(decoderId, Math.max(0, Math.round(targetMs)))
    return raw ? (JSON.parse(raw) as NativeStreamDecoderSnapshot) : null
  } catch {
    return null
  }
}

/** Optional snapshot query for native stream decoder. */
export function nativeStreamDecoderSnapshot(decoderId: string): NativeStreamDecoderSnapshot | null {
  try {
    const call = loadBinding()?.streamDecoderSnapshot
    if (typeof call !== 'function') return null
    const raw = call(decoderId)
    return raw ? (JSON.parse(raw) as NativeStreamDecoderSnapshot) : null
  } catch {
    return null
  }
}

/** Optional close and destroy of native stream decoder. */
export function nativeStreamDecoderClose(decoderId: string): boolean {
  try {
    const call = loadBinding()?.streamDecoderClose
    return typeof call === 'function' ? call(decoderId) === true : false
  } catch {
    return false
  }
}

/** Optional creation of native DSP pipeline. */
export function nativeDspPipelineCreate(
  pipelineId: string,
  sampleRate: number,
  channels: number,
  config?: Partial<NativeDspPipelineConfig> | null
): NativeDspPipelineSnapshot | null {
  try {
    const call = loadBinding()?.dspPipelineCreate
    if (typeof call !== 'function') return null
    const jsonStr = config ? JSON.stringify(config) : null
    const raw = call(
      pipelineId,
      Math.max(1, Math.round(sampleRate)),
      Math.max(1, Math.round(channels)),
      jsonStr
    )
    return raw ? (JSON.parse(raw) as NativeDspPipelineSnapshot) : null
  } catch {
    return null
  }
}

/** Optional configuration update for existing native DSP pipeline. */
export function nativeDspPipelineConfigure(
  pipelineId: string,
  config: NativeDspPipelineConfig
): NativeDspPipelineSnapshot | null {
  try {
    const call = loadBinding()?.dspPipelineConfigure
    if (typeof call !== 'function') return null
    const raw = call(pipelineId, JSON.stringify(config))
    return raw ? (JSON.parse(raw) as NativeDspPipelineSnapshot) : null
  } catch {
    return null
  }
}

/** Optional in-place DSP processing for interleaved PCM f32 buffer. */
export function nativeDspPipelineProcess(
  pipelineId: string,
  chunk: Uint8Array
): Uint8Array | null {
  try {
    const call = loadBinding()?.dspPipelineProcess
    if (typeof call !== 'function') return null
    return call(pipelineId, chunk) ?? null
  } catch {
    return null
  }
}

/** Optional state reset of native DSP pipeline. */
export function nativeDspPipelineReset(pipelineId: string): boolean {
  try {
    const call = loadBinding()?.dspPipelineReset
    return typeof call === 'function' ? call(pipelineId) === true : false
  } catch {
    return false
  }
}

/** Optional snapshot query of native DSP pipeline. */
export function nativeDspPipelineSnapshot(pipelineId: string): NativeDspPipelineSnapshot | null {
  try {
    const call = loadBinding()?.dspPipelineSnapshot
    if (typeof call !== 'function') return null
    const raw = call(pipelineId)
    return raw ? (JSON.parse(raw) as NativeDspPipelineSnapshot) : null
  } catch {
    return null
  }
}

/** Optional close and destroy of native DSP pipeline. */
export function nativeDspPipelineClose(pipelineId: string): boolean {
  try {
    const call = loadBinding()?.dspPipelineClose
    return typeof call === 'function' ? call(pipelineId) === true : false
  } catch {
    return false
  }
}

/** Optional creation of native streaming audio sink. */
export function nativeAudioSinkCreate(
  sinkId: string,
  cacheId: string,
  containerHint?: string | null,
  mimeHint?: string | null,
  dspConfig?: Partial<NativeDspPipelineConfig> | null,
  forceVirtual?: boolean | null
): NativeSinkSnapshot | null {
  try {
    const call = loadBinding()?.audioSinkCreate
    if (typeof call !== 'function') return null
    const dspStr = dspConfig ? JSON.stringify(dspConfig) : null
    const raw = call(
      sinkId,
      cacheId,
      containerHint ?? null,
      mimeHint ?? null,
      dspStr,
      forceVirtual ?? null
    )
    return raw ? (JSON.parse(raw) as NativeSinkSnapshot) : null
  } catch {
    return null
  }
}

/** Optional start or resume of native audio playback pump. */
export function nativeAudioSinkPlay(sinkId: string): boolean {
  try {
    const call = loadBinding()?.audioSinkPlay
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

/** Optional pause of native audio playback pump. */
export function nativeAudioSinkPause(sinkId: string): boolean {
  try {
    const call = loadBinding()?.audioSinkPause
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

/** Optional stop of native audio playback pump and buffer flush. */
export function nativeAudioSinkStop(sinkId: string): boolean {
  try {
    const call = loadBinding()?.audioSinkStop
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

/** Optional seek of native audio sink to target timestamp in milliseconds. */
export function nativeAudioSinkSeek(sinkId: string, targetMs: number): boolean {
  try {
    const call = loadBinding()?.audioSinkSeek
    return typeof call === 'function' ? call(sinkId, Math.max(0, Math.round(targetMs))) === true : false
  } catch {
    return false
  }
}

/** Optional master volume and mute control for native audio sink. */
export function nativeAudioSinkSetVolume(sinkId: string, volume: number, muted: boolean): boolean {
  try {
    const call = loadBinding()?.audioSinkSetVolume
    return typeof call === 'function' ? call(sinkId, volume, muted) === true : false
  } catch {
    return false
  }
}

/** Optional DSP effect configuration for native audio sink. */
export function nativeAudioSinkConfigureDsp(
  sinkId: string,
  config: NativeDspPipelineConfig
): NativeSinkSnapshot | null {
  try {
    const call = loadBinding()?.audioSinkConfigureDsp
    if (typeof call !== 'function') return null
    const raw = call(sinkId, JSON.stringify(config))
    return raw ? (JSON.parse(raw) as NativeSinkSnapshot) : null
  } catch {
    return null
  }
}

/** Optional snapshot query of native audio sink. */
export function nativeAudioSinkSnapshot(sinkId: string): NativeSinkSnapshot | null {
  try {
    const call = loadBinding()?.audioSinkSnapshot
    if (typeof call !== 'function') return null
    const raw = call(sinkId)
    return raw ? (JSON.parse(raw) as NativeSinkSnapshot) : null
  } catch {
    return null
  }
}

/** Optional close and destroy of native audio sink. */
export function nativeAudioSinkClose(sinkId: string): boolean {
  try {
    const call = loadBinding()?.audioSinkClose
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

/** Optional enumeration of host audio output devices. */
export function nativeAudioDevicesEnumerate(): NativeAudioDeviceInfo[] {
  try {
    const call = loadBinding()?.audioDevicesEnumerate
    if (typeof call !== 'function') return []
    const raw = call()
    return raw ? (JSON.parse(raw) as NativeAudioDeviceInfo[]) : []
  } catch {
    return []
  }
}

/** Optional dynamic device switching or exclusive mode toggle for native audio sink. */
export function nativeAudioSinkSwitchDevice(
  sinkId: string,
  deviceName?: string | null,
  exclusive?: boolean
): boolean {
  try {
    const call = loadBinding()?.audioSinkSwitchDevice
    return typeof call === 'function'
      ? call(sinkId, deviceName ?? null, exclusive ?? false) === true
      : false
  } catch {
    return false
  }
}

export interface NativeLyricMatch {
  index: number
  start: number
  end: number
  text: string
  progress: number
  prev_text?: string | null
  next_text?: string | null
}

export interface NativeDesktopLyricAppearance {
  bg_opacity: number
  text_opacity: number
  font_weight: number
  background_rgba: string
  text_opacity_str: string
  shadow_rgba: string
  stroke_width_px: number
}

export interface NativeMiniPlayerAppearance {
  opacity: number
  bg_rgba: string
  hover_rgba: string
  border_rgba: string
}

export interface NativeNetworkHeaders {
  provider: string
  target_url: string
  headers: Record<string, string>
}

export interface NativeNetworkStreamRange {
  start: number
  end: number
  length: number
  range_header: string
  is_final: boolean
}

/** Parse LRC text through shared Rust line-level parser with TypeScript fallback. */
export function nativeLyricsParse(content: string): NativeLyricLine[] {
  try {
    const raw = loadBinding()?.lyricsParseLrc?.(content)
    if (raw) return JSON.parse(raw) as NativeLyricLine[]
  } catch {
    // fallback
  }
  return []
}

/** Find active line and internal progress in lyrics timeline through Rust binary search. */
export function nativeLyricsFindActiveLine(
  lines: NativeLyricLine[],
  timeMs: number
): NativeLyricMatch | null {
  try {
    const raw = loadBinding()?.lyricsFindActiveLine?.(JSON.stringify(lines), timeMs)
    if (raw) return JSON.parse(raw) as NativeLyricMatch
  } catch {
    // fallback
  }
  if (!lines.length) return null
  const time = Math.max(0, timeMs)
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    if (time >= line.start && (line.end === 0 || time < line.end || i === lines.length - 1)) {
      const progress =
        line.end > line.start
          ? Math.max(0, Math.min(1, (time - line.start) / (line.end - line.start)))
          : 1
      return {
        index: i,
        start: line.start,
        end: line.end,
        text: line.text,
        progress,
        prev_text: i > 0 ? lines[i - 1].text : null,
        next_text: i + 1 < lines.length ? lines[i + 1].text : null
      }
    }
  }
  return null
}

/** Compute normalized appearance tokens and CSS variables for desktop lyrics. */
export function nativeDesktopLyricComputeAppearance(
  bgOpacity: number,
  textOpacity: number,
  fontWeight: number = 600,
  shadowOpacity: number = 0.72,
  strokeWidth: number = 0
): NativeDesktopLyricAppearance {
  try {
    const raw = loadBinding()?.desktopLyricComputeAppearance?.(
      bgOpacity,
      textOpacity,
      fontWeight,
      shadowOpacity,
      strokeWidth
    )
    if (raw) return JSON.parse(raw) as NativeDesktopLyricAppearance
  } catch {
    // fallback
  }
  const bg = Math.max(0, Math.min(1, bgOpacity))
  const text = Math.max(0.06, Math.min(1, textOpacity / 100))
  return {
    bg_opacity: bg,
    text_opacity: text,
    font_weight: Math.max(100, Math.min(900, fontWeight)),
    background_rgba: `rgba(9, 12, 11, ${bg.toFixed(2)})`,
    text_opacity_str: text.toFixed(2),
    shadow_rgba: `rgba(0, 0, 0, ${shadowOpacity.toFixed(2)})`,
    stroke_width_px: strokeWidth
  }
}

/** Compute normalized appearance tokens and background RGBA for mini player. */
export function nativeMiniPlayerComputeAppearance(
  opacityPercent: number
): NativeMiniPlayerAppearance {
  try {
    const raw = loadBinding()?.miniPlayerComputeAppearance?.(opacityPercent)
    if (raw) return JSON.parse(raw) as NativeMiniPlayerAppearance
  } catch {
    // fallback
  }
  const op = Math.max(0.1, Math.min(1, opacityPercent / 100))
  return {
    opacity: op,
    bg_rgba: `rgba(31, 33, 31, ${op.toFixed(2)})`,
    hover_rgba: `rgba(35, 37, 34, ${Math.min(1, op + 0.18).toFixed(2)})`,
    border_rgba: 'rgba(255, 255, 255, 0.20)'
  }
}

/** Generate platform-specific headers for online stream and lyric requests. */
export function nativeNetworkBuildHeaders(
  provider: string,
  targetUrl: string,
  rangeStart?: number | null,
  rangeEnd?: number | null
): NativeNetworkHeaders {
  try {
    const raw = loadBinding()?.networkBuildHeaders?.(provider, targetUrl, rangeStart, rangeEnd)
    if (raw) return JSON.parse(raw) as NativeNetworkHeaders
  } catch {
    // fallback
  }
  const headers: Record<string, string> = {
    Accept: '*/*',
    'Connection': 'keep-alive',
    'User-Agent':
      'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36'
  }
  if (provider === 'qq') {
    headers['Referer'] = 'https://y.qq.com/'
  } else if (provider === 'wy') {
    headers['Referer'] = 'https://music.163.com/'
  }
  if (rangeStart != null) {
    headers['Range'] = rangeEnd != null ? `bytes=${rangeStart}-${rangeEnd}` : `bytes=${rangeStart}-`
  }
  return {
    provider,
    target_url: targetUrl,
    headers
  }
}

/** Plan chunk ranges for remote stream caching and range requests. */
export function nativeNetworkPlanStreamRange(
  startByte: number,
  totalBytes: number,
  chunkSize: number = 256 * 1024
): NativeNetworkStreamRange {
  try {
    const raw = loadBinding()?.networkPlanStreamRange?.(startByte, totalBytes, chunkSize)
    if (raw) return JSON.parse(raw) as NativeNetworkStreamRange
  } catch {
    // fallback
  }
  const step = Math.max(64 * 1024, chunkSize)
  const end = totalBytes > 0 ? Math.min(startByte + step - 1, totalBytes - 1) : startByte + step - 1
  return {
    start: startByte,
    end,
    length: end - startByte + 1,
    range_header: `bytes=${startByte}-${end}`,
    is_final: totalBytes > 0 ? end + 1 >= totalBytes : false
  }
}

/** Validate and normalize proxy URI string through Rust network module. */
export function nativeNetworkValidateProxy(
  enable: boolean,
  host: string,
  port: number
): string | null {
  try {
    const raw = loadBinding()?.networkValidateProxy?.(enable, host, port)
    return raw || null
  } catch {
    if (!enable || !host) return null
    return `http://${host}:${port}`
  }
}

export interface NativeSyllableSpan {
  text: string
  start_ms: number
  duration_ms: number
}

export interface NativeSyllableLine {
  start_ms: number
  end_ms: number
  raw_text: string
  syllables: NativeSyllableSpan[]
}

export interface NativeSyllableProgress {
  current_syllable_index: number
  syllable_progress: number
  line_progress: number
  sang_text: string
  current_text: string
  unsang_text: string
}

export interface NativeDesktopLyricRenderFrame {
  main_text: string
  sub_text?: string | null
  highlight_ratio: number
  active_color: string
  normal_color: string
  background_rgba: string
  font_size: number
  font_weight: number
  is_playing: boolean
}

/** Parse full LRC into syllable lines with word-level timestamps via Rust. */
export function nativeLyricsParseSyllables(lrcText: string): NativeSyllableLine[] {
  try {
    const raw = loadBinding()?.lyricsParseSyllables?.(lrcText)
    if (raw) return JSON.parse(raw) as NativeSyllableLine[]
  } catch {
    // fallback
  }
  return []
}

/** Compute real-time karaoke syllable progress within a line via Rust. */
export function nativeLyricsComputeSyllableProgress(
  line: NativeSyllableLine,
  timeMs: number
): NativeSyllableProgress {
  try {
    const raw = loadBinding()?.lyricsComputeSyllableProgress?.(JSON.stringify(line), timeMs)
    if (raw) return JSON.parse(raw) as NativeSyllableProgress
  } catch {
    // fallback
  }
  return {
    current_syllable_index: 0,
    syllable_progress: 0,
    line_progress: 0,
    sang_text: '',
    current_text: '',
    unsang_text: line.raw_text
  }
}

/** Render structured desktop lyric frame with typography and appearance via Rust. */
export function nativeLyricsRenderDesktopFrame(
  mainLine: string,
  subLine: string | null | undefined,
  progress: number,
  activeColor: string,
  normalColor: string,
  bgOpacity: number,
  textOpacity: number,
  fontSize: number,
  isPlaying: boolean
): NativeDesktopLyricRenderFrame {
  try {
    const raw = loadBinding()?.lyricsRenderDesktopFrame?.(
      mainLine,
      subLine,
      progress,
      activeColor,
      normalColor,
      bgOpacity,
      textOpacity,
      fontSize,
      isPlaying
    )
    if (raw) return JSON.parse(raw) as NativeDesktopLyricRenderFrame
  } catch {
    // fallback
  }
  return {
    main_text: mainLine,
    sub_text: subLine,
    highlight_ratio: Math.max(0, Math.min(1, progress)),
    active_color: activeColor,
    normal_color: normalColor,
    background_rgba: `rgba(9, 12, 11, ${bgOpacity.toFixed(2)})`,
    font_size: fontSize,
    font_weight: 600,
    is_playing: isPlaying
  }
}

export interface NativeMatchTags {
  title: string
  artist: string
}

export interface NativeScoredCandidate<T = unknown> {
  item: T
  score: number
  titleMatched: boolean
  artistMatched: boolean
  durationDiffMs: number
}

/** Normalize music text using Rust unicode-aware normalizer. */
export function nativeMatcherNormalizeMusicText(text: string): string {
  try {
    const raw = loadBinding()?.matcherNormalizeMusicText?.(text)
    if (typeof raw === 'string') return raw
  } catch {
    // fallback
  }
  return text
    .normalize('NFKC')
    .toLocaleLowerCase()
    .replace(/\b(?:feat|ft)\.?\b/gu, '')
    .replace(/[^\p{L}\p{N}]+/gu, '')
}

/** Check if two artist strings match (equality or containment). */
export function nativeMatcherArtistsMatch(left: string, right: string): boolean {
  try {
    const res = loadBinding()?.matcherArtistsMatch?.(left, right)
    if (typeof res === 'boolean') return res
  } catch {
    // fallback
  }
  const a = nativeMatcherNormalizeMusicText(left)
  const b = nativeMatcherNormalizeMusicText(right)
  return !!a && !!b && (a === b || a.includes(b) || b.includes(a))
}

/** Select a single unambiguous enrichment candidate from a list. */
export function nativeMatcherSelectEnrichmentCandidate<
  T extends { type?: string; title: string; artist: string; duration?: number }
>(tags: NativeMatchTags, duration: number, candidates: T[]): T | null {
  try {
    const call = loadBinding()?.matcherSelectEnrichmentCandidate
    if (typeof call === 'function') {
      const raw = call(
        JSON.stringify(tags),
        Math.max(0, Math.round(duration)),
        JSON.stringify(candidates)
      )
      if (raw) return JSON.parse(raw) as T
      return null
    }
  } catch {
    // fallback
  }
  const title = nativeMatcherNormalizeMusicText(tags.title)
  if (!title || !nativeMatcherNormalizeMusicText(tags.artist)) return null
  const matches = candidates.filter((candidate) => {
    if (candidate.type === 'local' || nativeMatcherNormalizeMusicText(candidate.title) !== title)
      return false
    if (!nativeMatcherArtistsMatch(tags.artist, candidate.artist)) return false
    const candDur = candidate.duration ?? 0
    if (duration > 0 && candDur > 0 && Math.abs(duration - candDur) > 5000) return false
    return true
  })
  return matches.length === 1 ? matches[0] : null
}

/** Compute fuzzy match score between local track tags and an online candidate. */
export function nativeMatcherScoreMusicMatch(
  tagTitle: string,
  tagArtist: string,
  tagDuration: number,
  candidateTitle: string,
  candidateArtist: string,
  candidateDuration: number
): number {
  try {
    const score = loadBinding()?.matcherScoreMusicMatch?.(
      tagTitle,
      tagArtist,
      Math.max(0, Math.round(tagDuration)),
      candidateTitle,
      candidateArtist,
      Math.max(0, Math.round(candidateDuration))
    )
    if (typeof score === 'number' && Number.isFinite(score)) return score
  } catch {
    // fallback
  }
  const tNorm = nativeMatcherNormalizeMusicText(tagTitle)
  const cNorm = nativeMatcherNormalizeMusicText(candidateTitle)
  if (!tNorm || !cNorm) return 0
  let score = 0
  if (tNorm === cNorm) score += 0.6
  else if (tNorm.includes(cNorm) || cNorm.includes(tNorm)) score += 0.4
  const aNorm = nativeMatcherNormalizeMusicText(tagArtist)
  const caNorm = nativeMatcherNormalizeMusicText(candidateArtist)
  if (aNorm && caNorm) {
    if (aNorm === caNorm) score += 0.3
    else if (aNorm.includes(caNorm) || caNorm.includes(aNorm)) score += 0.2
  }
  if (tagDuration > 0 && candidateDuration > 0) {
    const diff = Math.abs(tagDuration - candidateDuration)
    if (diff <= 2000) score += 0.1
    else if (diff <= 5000) score += 0.05
    else if (diff > 15000) score -= 0.15
  }
  return Math.max(0, Math.min(1, score))
}

/** Rank online candidates by relevance score against local track tags. */
export function nativeMatcherRankCandidates<
  T extends { type?: string; title: string; artist: string; duration?: number }
>(tags: NativeMatchTags, duration: number, candidates: T[]): Array<NativeScoredCandidate<T>> {
  try {
    const call = loadBinding()?.matcherRankCandidates
    if (typeof call === 'function') {
      const raw = call(
        JSON.stringify(tags),
        Math.max(0, Math.round(duration)),
        JSON.stringify(candidates)
      )
      if (raw) return JSON.parse(raw) as Array<NativeScoredCandidate<T>>
    }
  } catch {
    // fallback
  }
  return candidates
    .filter((c) => c.type !== 'local')
    .map((item) => {
      const score = nativeMatcherScoreMusicMatch(
        tags.title,
        tags.artist,
        duration,
        item.title,
        item.artist,
        item.duration ?? 0
      )
      return {
        item,
        score,
        titleMatched:
          nativeMatcherNormalizeMusicText(item.title) ===
          nativeMatcherNormalizeMusicText(tags.title),
        artistMatched: nativeMatcherArtistsMatch(tags.artist, item.artist),
        durationDiffMs:
          duration > 0 && (item.duration ?? 0) > 0 ? Math.abs(duration - (item.duration ?? 0)) : 0
      }
    })
    .sort((a, b) => b.score - a.score)
}

/** Sniff real container audio extension from file header (.ogg, .flac, .mp3). */
export function nativeDownloadSniffAudioExtension(filePath: string): string | null {
  try {
    const raw = loadBinding()?.downloadSniffAudioExtension?.(filePath)
    if (typeof raw === 'string') return raw
  } catch {
    // fallback
  }
  return null
}

/** Parse FLAC sample rate and bit depth tag from STREAMINFO header (e.g. [16Bit-44.1kHz]). */
export function nativeDownloadParseFlacQualityTag(filePath: string): string | null {
  try {
    const raw = loadBinding()?.downloadParseFlacQualityTag?.(filePath)
    if (typeof raw === 'string') return raw
  } catch {
    // fallback
  }
  return null
}

/** Normalize download title for cross-source alternative candidate search. */
export function nativeDownloadNormalizeTitle(title: string): string {
  try {
    const raw = loadBinding()?.downloadNormalizeTitle?.(title)
    if (typeof raw === 'string') return raw
  } catch {
    // fallback
  }
  return title.toLowerCase().replace(/[\s()（）【】\-—_·,.，。'"'""?!:：;；]/g, '')
}

/** Search alternative candidates for first matching title. */
export function nativeDownloadSelectAlternativeCandidate<T extends { title: string }>(
  targetTitle: string,
  candidates: T[]
): T | null {
  try {
    const call = loadBinding()?.downloadSelectAlternativeCandidate
    if (typeof call === 'function') {
      const raw = call(targetTitle, JSON.stringify(candidates))
      if (raw) return JSON.parse(raw) as T
    }
  } catch {
    // fallback
  }
  const target = nativeDownloadNormalizeTitle(targetTitle)
  if (!target) return null
  return candidates.find((c) => nativeDownloadNormalizeTitle(c.title) === target) ?? null
}
