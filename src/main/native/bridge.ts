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
}

export type NativePlaybackSnapshot = {
  trackId: string | null
  status: 'idle' | 'paused' | 'playing' | 'ended'
  positionMs: number
  durationMs: number
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
