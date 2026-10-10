/* eslint-disable @typescript-eslint/explicit-function-return-type */
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

const require = createRequire(import.meta.url)
let binding

function nativeCandidates() {
  const file = 'aurora-native.win32-x64-msvc.node'
  const candidates = []
  if (typeof process.resourcesPath === 'string')
    candidates.push(join(process.resourcesPath, 'assets', file))
  candidates.push(join(process.cwd(), 'crates', 'aurora-native', file))
  if (typeof __dirname === 'string')
    candidates.push(join(__dirname, '..', '..', 'crates', 'aurora-native', file))
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

export function nativeThemeMotionProfile(themeId, reducedMotion, requestedFps, displayRefreshHz) {
  try {
    const call = loadBinding()?.themeMotionProfile
    if (typeof call !== 'function') return null
    const raw = call(
      themeId,
      !!reducedMotion,
      requestedFps == null ? null : Number(requestedFps),
      displayRefreshHz == null ? null : Number(displayRefreshHz)
    )
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
  return nativeAudioCall(
    'nativeAudioSetVolume',
    Math.max(0, Math.min(1, Number(volume) || 0)),
    !!muted
  )
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

export function nativeRemoteStreamNegotiateFormat(url, contentTypeHint, upstreamContentType, ekey) {
  try {
    const call = loadBinding()?.remoteStreamNegotiateFormat
    if (typeof call !== 'function') return null
    const raw = call(url, contentTypeHint ?? null, upstreamContentType ?? null, ekey ?? null)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemotePlaybackShadowCreate(
  sessionId,
  url,
  contentTypeHint,
  ekey,
  durationMs = 0
) {
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
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemotePlaybackShadowUpdateResponse(
  sessionId,
  statusCode,
  contentLength,
  contentRange,
  upstreamContentType
) {
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
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemotePlaybackShadowReceiveChunk(sessionId, offset, length) {
  try {
    const call = loadBinding()?.remotePlaybackShadowReceiveChunk
    if (typeof call !== 'function') return null
    const raw = call(sessionId, Math.max(0, Math.round(offset)), Math.max(0, Math.round(length)))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemotePlaybackShadowSeek(sessionId, targetMs, durationMs = 0) {
  try {
    const call = loadBinding()?.remotePlaybackShadowSeek
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      Math.max(0, Math.round(targetMs)),
      Math.max(0, Math.round(durationMs))
    )
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemotePlaybackShadowSetPlayState(sessionId, playing) {
  try {
    const call = loadBinding()?.remotePlaybackShadowSetPlayState
    if (typeof call !== 'function') return null
    const raw = call(sessionId, !!playing)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemotePlaybackShadowSnapshot(sessionId) {
  try {
    const call = loadBinding()?.remotePlaybackShadowSnapshot
    if (typeof call !== 'function') return null
    const raw = call(sessionId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemotePlaybackShadowClose(sessionId) {
  try {
    const call = loadBinding()?.remotePlaybackShadowClose
    return typeof call === 'function' ? call(sessionId) === true : false
  } catch {
    return false
  }
}

export function nativeRemoteStreamPlanRange(sessionId, currentOffset, chunkSize) {
  try {
    const call = loadBinding()?.remoteStreamPlanRange
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      currentOffset == null ? null : Math.max(0, Math.round(Number(currentOffset))),
      chunkSize == null ? null : Math.max(0, Math.round(Number(chunkSize)))
    )
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemoteStreamGetBufferedRanges(sessionId) {
  try {
    const call = loadBinding()?.remoteStreamGetBufferedRanges
    if (typeof call !== 'function') return null
    const raw = call(sessionId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeRemoteStreamIsOffsetBuffered(sessionId, offset) {
  try {
    const call = loadBinding()?.remoteStreamIsOffsetBuffered
    if (typeof call !== 'function') return false
    return call(sessionId, Math.max(0, Math.round(Number(offset)))) === true
  } catch {
    return false
  }
}

export function nativeRemoteStreamCalculateSeekRange(
  sessionId,
  targetMs,
  durationMs = 0,
  alignBytes
) {
  try {
    const call = loadBinding()?.remoteStreamCalculateSeekRange
    if (typeof call !== 'function') return null
    const raw = call(
      sessionId,
      Math.max(0, Math.round(Number(targetMs))),
      Math.max(0, Math.round(Number(durationMs))),
      alignBytes == null ? null : Math.max(0, Math.round(Number(alignBytes)))
    )
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamCacheCreate(cacheId, maxMemoryBytes, tempDir) {
  try {
    const call = loadBinding()?.streamCacheCreate
    if (typeof call !== 'function') return null
    const raw = call(
      cacheId,
      maxMemoryBytes == null ? null : Math.max(0, Math.round(Number(maxMemoryBytes))),
      tempDir ?? null
    )
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamCacheWriteChunk(cacheId, offset, chunk) {
  try {
    const call = loadBinding()?.streamCacheWriteChunk
    if (typeof call !== 'function') return 0
    const written = call(cacheId, Math.max(0, Math.round(Number(offset))), chunk)
    return Number.isSafeInteger(written) && written > 0 ? written : 0
  } catch {
    return 0
  }
}

export function nativeStreamCacheRead(cacheId, offset, maxLen) {
  try {
    const call = loadBinding()?.streamCacheRead
    if (typeof call !== 'function') return null
    const output = call(
      cacheId,
      Math.max(0, Math.round(Number(offset))),
      Math.max(0, Math.round(Number(maxLen)))
    )
    return output instanceof Uint8Array ? output : null
  } catch {
    return null
  }
}

export function nativeStreamCacheStats(cacheId) {
  try {
    const call = loadBinding()?.streamCacheStats
    if (typeof call !== 'function') return null
    const raw = call(cacheId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamCacheSetTotalLength(cacheId, totalLength) {
  try {
    const call = loadBinding()?.streamCacheSetTotalLength
    return typeof call === 'function'
      ? call(cacheId, Math.max(0, Math.round(Number(totalLength)))) === true
      : false
  } catch {
    return false
  }
}

export function nativeStreamCacheClose(cacheId) {
  try {
    const call = loadBinding()?.streamCacheClose
    return typeof call === 'function' ? call(cacheId) === true : false
  } catch {
    return false
  }
}

export function nativeStreamDecoderCreate(decoderId, cacheId, containerHint, mimeHint) {
  try {
    const call = loadBinding()?.streamDecoderCreate
    if (typeof call !== 'function') return null
    const raw = call(decoderId, cacheId, containerHint ?? null, mimeHint ?? null)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamDecoderProbe(decoderId) {
  try {
    const call = loadBinding()?.streamDecoderProbe
    if (typeof call !== 'function') return null
    const raw = call(decoderId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamDecoderStep(decoderId) {
  try {
    const call = loadBinding()?.streamDecoderStep
    if (typeof call !== 'function') return null
    const raw = call(decoderId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamDecoderSeek(decoderId, targetMs) {
  try {
    const call = loadBinding()?.streamDecoderSeek
    if (typeof call !== 'function') return null
    const raw = call(decoderId, Math.max(0, Math.round(Number(targetMs))))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamDecoderSnapshot(decoderId) {
  try {
    const call = loadBinding()?.streamDecoderSnapshot
    if (typeof call !== 'function') return null
    const raw = call(decoderId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeStreamDecoderClose(decoderId) {
  try {
    const call = loadBinding()?.streamDecoderClose
    return typeof call === 'function' ? call(decoderId) === true : false
  } catch {
    return false
  }
}

export function nativeDspPipelineCreate(pipelineId, sampleRate, channels, config) {
  try {
    const call = loadBinding()?.dspPipelineCreate
    if (typeof call !== 'function') return null
    const jsonStr = config ? JSON.stringify(config) : null
    const raw = call(
      pipelineId,
      Math.max(1, Math.round(Number(sampleRate))),
      Math.max(1, Math.round(Number(channels))),
      jsonStr
    )
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeDspPipelineConfigure(pipelineId, config) {
  try {
    const call = loadBinding()?.dspPipelineConfigure
    if (typeof call !== 'function') return null
    const raw = call(pipelineId, JSON.stringify(config))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeDspPipelineProcess(pipelineId, chunk) {
  try {
    const call = loadBinding()?.dspPipelineProcess
    if (typeof call !== 'function') return null
    return call(pipelineId, chunk) ?? null
  } catch {
    return null
  }
}

export function nativeDspPipelineReset(pipelineId) {
  try {
    const call = loadBinding()?.dspPipelineReset
    return typeof call === 'function' ? call(pipelineId) === true : false
  } catch {
    return false
  }
}

export function nativeDspPipelineSnapshot(pipelineId) {
  try {
    const call = loadBinding()?.dspPipelineSnapshot
    if (typeof call !== 'function') return null
    const raw = call(pipelineId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeDspPipelineClose(pipelineId) {
  try {
    const call = loadBinding()?.dspPipelineClose
    return typeof call === 'function' ? call(pipelineId) === true : false
  } catch {
    return false
  }
}

export function nativeAudioSinkCreate(
  sinkId,
  cacheId,
  containerHint,
  mimeHint,
  dspConfig,
  forceVirtual
) {
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
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeAudioSinkPlay(sinkId) {
  try {
    const call = loadBinding()?.audioSinkPlay
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

export function nativeAudioSinkPause(sinkId) {
  try {
    const call = loadBinding()?.audioSinkPause
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

export function nativeAudioSinkStop(sinkId) {
  try {
    const call = loadBinding()?.audioSinkStop
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

export function nativeAudioSinkSeek(sinkId, targetMs) {
  try {
    const call = loadBinding()?.audioSinkSeek
    return typeof call === 'function'
      ? call(sinkId, Math.max(0, Math.round(Number(targetMs)))) === true
      : false
  } catch {
    return false
  }
}

export function nativeAudioSinkSetVolume(sinkId, volume, muted) {
  try {
    const call = loadBinding()?.audioSinkSetVolume
    return typeof call === 'function' ? call(sinkId, Number(volume), Boolean(muted)) === true : false
  } catch {
    return false
  }
}

export function nativeAudioSinkConfigureDsp(sinkId, config) {
  try {
    const call = loadBinding()?.audioSinkConfigureDsp
    if (typeof call !== 'function') return null
    const raw = call(sinkId, JSON.stringify(config))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeAudioSinkSnapshot(sinkId) {
  try {
    const call = loadBinding()?.audioSinkSnapshot
    if (typeof call !== 'function') return null
    const raw = call(sinkId)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeAudioSinkClose(sinkId) {
  try {
    const call = loadBinding()?.audioSinkClose
    return typeof call === 'function' ? call(sinkId) === true : false
  } catch {
    return false
  }
}

export function nativeAudioDevicesEnumerate() {
  try {
    const call = loadBinding()?.audioDevicesEnumerate
    if (typeof call !== 'function') return []
    const raw = call()
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

export function nativeAudioSinkSwitchDevice(sinkId, deviceName, exclusive) {
  try {
    const call = loadBinding()?.audioSinkSwitchDevice
    return typeof call === 'function'
      ? call(sinkId, deviceName ?? null, Boolean(exclusive)) === true
      : false
  } catch {
    return false
  }
}

export function nativeParseKgTrack(item) {
  try {
    const call = loadBinding()?.providerParseKgTrack
    if (typeof call !== 'function') return null
    const raw = call(JSON.stringify(item))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeParseKwTrack(item) {
  try {
    const call = loadBinding()?.providerParseKwTrack
    if (typeof call !== 'function') return null
    const raw = call(JSON.stringify(item))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeKgSign(params, body = '') {
  try {
    const call = loadBinding()?.providerKgSign
    if (typeof call !== 'function') return null
    return call(JSON.stringify(params), body)
  } catch {
    return null
  }
}

export function nativeExtractBackendMusicId(platform, id, mid, hash) {
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

export function nativeSelectOptimalQuality(availableQualities, preferredQuality) {
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

export function nativeParseMediaInfoResponse(source, qualityId, response, nowMs = Date.now()) {
  try {
    const call = loadBinding()?.providerParseMediaInfoResponse
    if (typeof call !== 'function') return null
    const raw = call(source, qualityId, JSON.stringify(response), nowMs)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeParsePlaylistEntry(source, entry) {
  try {
    const call = loadBinding()?.providerParsePlaylistEntry
    if (typeof call !== 'function') return null
    const raw = call(source, JSON.stringify(entry))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeParseAlbumEntry(source, entry) {
  try {
    const call = loadBinding()?.providerParseAlbumEntry
    if (typeof call !== 'function') return null
    const raw = call(source, JSON.stringify(entry))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

export function nativeParseArtistEntry(source, entry) {
  try {
    const call = loadBinding()?.providerParseArtistEntry
    if (typeof call !== 'function') return null
    const raw = call(source, JSON.stringify(entry))
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}
