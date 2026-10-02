/* eslint-disable @typescript-eslint/explicit-function-return-type */
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

const require = createRequire(import.meta.url)
let binding

function nativeCandidates() {
  const file = 'aurora-native.win32-x64-msvc.node'
  const candidates = []
  if (typeof process.resourcesPath === 'string') candidates.push(join(process.resourcesPath, 'assets', file))
  candidates.push(join(process.cwd(), 'crates', 'aurora-native', file))
  if (typeof __dirname === 'string') candidates.push(join(__dirname, '..', '..', 'crates', 'aurora-native', file))
  return candidates
}

function loadBinding() {
  if (binding !== undefined) return binding
  for (const candidate of nativeCandidates()) {
    if (!existsSync(candidate)) continue
    try {
      binding = require(candidate)
      return binding
    } catch {
      // Keep the TypeScript bridge's optional-native fallback semantics.
    }
  }
  binding = null
  return null
}

export function nativeReadAudioTags(path) {
  try {
    const raw = loadBinding()?.readAudioTags(path)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

/**
 * Best-effort native tag writer.  The N-API function receives one JSON
 * document so the bridge remains compatible with the existing JSON DTOs.
 * Buffer#toJSON is normalised back to a byte array before serialisation;
 * Rust's serde DTO therefore sees `pictureData` as `Vec<u8>`.
 */
export function nativeWriteAudioTags(path, metadata) {
  try {
    const write = loadBinding()?.writeAudioTags
    if (typeof write !== 'function') return false
    const json = JSON.stringify(metadata, (_key, value) => {
      if (value && value.type === 'Buffer' && Array.isArray(value.data)) return value.data
      return value
    })
    const result = write(path, json)
    return result !== false && result !== 'false'
  } catch {
    return false
  }
}

function nativePlaybackCall(method, ...args) {
  try {
    const call = loadBinding()?.[method]
    if (typeof call !== 'function') return null
    const raw = call(...args)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativePlaybackSnapshot() {
  return nativePlaybackCall('playbackSnapshot')
}

export function nativePlaybackLoad(trackId, durationMs) {
  return nativePlaybackCall('playbackLoad', trackId, Math.max(0, Math.round(durationMs)))
}

export function nativePlaybackPlay() {
  return nativePlaybackCall('playbackPlay')
}

export function nativePlaybackPause() {
  return nativePlaybackCall('playbackPause')
}

export function nativePlaybackSeek(positionMs) {
  return nativePlaybackCall('playbackSeek', Math.max(0, Math.round(positionMs)))
}

export function nativePlaybackTick(elapsedMs) {
  return nativePlaybackCall('playbackTick', Math.max(0, Math.round(elapsedMs)))
}

export function nativePlaybackStop() {
  return nativePlaybackCall('playbackStop')
}

export function nativeAudioBackendCapabilities() {
  try {
    const call = loadBinding()?.audioBackendCapabilities
    if (typeof call !== 'function') return null
    const raw = call()
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeThemeMotionProfile(themeId, reducedMotion) {
  try {
    const call = loadBinding()?.themeMotionProfile
    if (typeof call !== 'function') return null
    const raw = call(themeId, !!reducedMotion)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

function nativeAudioCall(method, ...args) {
  try {
    const call = loadBinding()?.[method]
    if (typeof call !== 'function') return null
    const raw = call(...args)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeAudioStartFile(path) {
  return nativeAudioCall('nativeAudioStartFile', path)
}

export function nativeAudioPlay() {
  return nativeAudioCall('nativeAudioPlay')
}

export function nativeAudioPause() {
  return nativeAudioCall('nativeAudioPause')
}

export function nativeAudioSetVolume(volume, muted) {
  return nativeAudioCall('nativeAudioSetVolume', Math.max(0, Math.min(1, Number(volume) || 0)), !!muted)
}

export function nativeAudioSeek(positionMs) {
  return nativeAudioCall('nativeAudioSeek', Math.max(0, Math.round(positionMs)))
}

export function nativeAudioStop() {
  return nativeAudioCall('nativeAudioStop')
}

export function nativeAudioSnapshot() {
  return nativeAudioCall('nativeAudioSnapshot')
}

/** Optional Rust QMC2 chunk path; callers keep the TypeScript fallback. */
export function nativeAudioDecryptQmc2Chunk(ekey, fileOffset, chunk) {
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
export function nativeAudioStreamCreate(ekey) {
  try {
    const call = loadBinding()?.audioStreamCreate
    if (typeof call !== 'function') return null
    const id = call(ekey)
    return Number.isSafeInteger(id) && id > 0 ? id : null
  } catch {
    return null
  }
}

export function nativeAudioStreamDecrypt(id, fileOffset, chunk) {
  try {
    const call = loadBinding()?.audioStreamDecrypt
    if (typeof call !== 'function') return null
    const output = call(id, Math.max(0, Math.round(fileOffset)), chunk)
    return output instanceof Uint8Array ? output : null
  } catch {
    return null
  }
}

export function nativeAudioStreamClose(id) {
  try {
    const call = loadBinding()?.audioStreamClose
    return typeof call === 'function' ? call(id) === true : false
  } catch {
    return false
  }
}

/** Optional bounded Rust download transaction; the host keeps HTTP/proxy ownership. */
export function nativeDownloadCreate(tempPath, finalPath, maxBytes) {
  try {
    const call = loadBinding()?.downloadCreate
    if (typeof call !== 'function') return null
    const id = call(tempPath, finalPath, Math.max(0, Math.round(maxBytes)))
    return Number.isSafeInteger(id) && id > 0 ? id : null
  } catch {
    return null
  }
}

export function nativeDownloadWrite(id, chunk) {
  try {
    const call = loadBinding()?.downloadWrite
    if (typeof call !== 'function') return null
    const raw = call(id, chunk)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeDownloadCommit(id) {
  try {
    const call = loadBinding()?.downloadCommit
    if (typeof call !== 'function') return null
    const raw = call(id)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeDownloadAbort(id) {
  try {
    const call = loadBinding()?.downloadAbort
    return typeof call === 'function' ? call(id) === true : false
  } catch {
    return false
  }
}

/** Optional Rust bounded-buffer path for completed QMC2 downloads. */
export function nativeAudioDecryptQmc2File(path, ekey) {
  try {
    const call = loadBinding()?.audioDecryptQmc2File
    if (typeof call !== 'function') return false
    return call(path, ekey) === true
  } catch {
    return false
  }
}
