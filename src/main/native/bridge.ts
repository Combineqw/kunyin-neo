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
  nativeAudioStartFile?(path: string): string
  nativeAudioPlay?(): string
  nativeAudioPause?(): string
  nativeAudioSetVolume?(volume: number, muted: boolean): string
  nativeAudioSeek?(positionMs: number): string
  nativeAudioStop?(): string
  nativeAudioSnapshot?(): string
  audioDecryptQmc2Chunk?(ekey: string, fileOffset: number, chunk: Uint8Array): Uint8Array
  audioDecryptQmc2File?(path: string, ekey: string): boolean
  syncNormalizeBaseUrl?(url: string): string
  syncValidateSession?(sessionJson: string): string
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

function nativePlaybackCall(method: keyof Pick<NativeBinding, 'playbackSnapshot' | 'playbackLoad' | 'playbackPlay' | 'playbackPause' | 'playbackSeek' | 'playbackTick' | 'playbackStop'>, ...args: unknown[]): NativePlaybackSnapshot | null {
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

export function nativePlaybackLoad(trackId: string, durationMs: number): NativePlaybackSnapshot | null {
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
