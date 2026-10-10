import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import {
  nativeRemoteStreamNegotiateFormat,
  nativeRemotePlaybackShadowCreate,
  nativeRemotePlaybackShadowUpdateResponse,
  nativeRemotePlaybackShadowReceiveChunk,
  nativeRemotePlaybackShadowSeek,
  nativeRemotePlaybackShadowSetPlayState,
  nativeRemotePlaybackShadowSnapshot,
  nativeRemotePlaybackShadowClose
} from '../src/main/native/bridge-runtime.js'

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M37 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

// 1. Direct native export presence
assert.equal(typeof native.remoteStreamNegotiateFormat, 'function')
assert.equal(typeof native.remotePlaybackShadowCreate, 'function')
assert.equal(typeof native.remotePlaybackShadowUpdateResponse, 'function')
assert.equal(typeof native.remotePlaybackShadowReceiveChunk, 'function')
assert.equal(typeof native.remotePlaybackShadowSeek, 'function')
assert.equal(typeof native.remotePlaybackShadowSetPlayState, 'function')
assert.equal(typeof native.remotePlaybackShadowSnapshot, 'function')
assert.equal(typeof native.remotePlaybackShadowClose, 'function')

// 2. Format negotiation
const fmtQq = JSON.parse(
  native.remoteStreamNegotiateFormat('http://music.tc.qq.com/C40000test.mflac?vkey=123', null, 'audio/mpeg', null)
)
assert.equal(fmtQq.normalizedUrl, 'https://music.tc.qq.com/C40000test.mflac?vkey=123')
assert.equal(fmtQq.mimeType, 'audio/flac')
assert.equal(fmtQq.container, 'flac')
assert.equal(fmtQq.encryption, 'qmc2')
assert.equal(fmtQq.sourceHint, 'url_extension')

const fmtMp3 = JSON.parse(
  native.remoteStreamNegotiateFormat('https://example.com/audio/track.mp3', null, null, null)
)
assert.equal(fmtMp3.mimeType, 'audio/mpeg')
assert.equal(fmtMp3.container, 'mp3')
assert.equal(fmtMp3.encryption, 'none')
assert.equal(fmtMp3.sourceHint, 'url_extension')

const fmtUpstream = JSON.parse(
  native.remoteStreamNegotiateFormat('https://example.com/audio/stream', null, 'audio/ogg', null)
)
assert.equal(fmtUpstream.mimeType, 'audio/ogg')
assert.equal(fmtUpstream.container, 'ogg')
assert.equal(fmtUpstream.sourceHint, 'upstream_header')

const fmtExplicit = JSON.parse(
  native.remoteStreamNegotiateFormat('https://example.com/audio/stream', 'audio/wav', null, null)
)
assert.equal(fmtExplicit.mimeType, 'audio/wav')
assert.equal(fmtExplicit.container, 'wav')
assert.equal(fmtExplicit.sourceHint, 'explicit')

const fmtFallback = JSON.parse(
  native.remoteStreamNegotiateFormat('https://example.com/audio/stream', null, null, null)
)
assert.equal(fmtFallback.mimeType, 'audio/mpeg')
assert.equal(fmtFallback.container, 'mp3')
assert.equal(fmtFallback.sourceHint, 'fallback')

// 3. Shadow session lifecycle via direct N-API
const sessionId = 'm37-test-session'
const rawInit = native.remotePlaybackShadowCreate(
  sessionId,
  'http://music.tc.qq.com/track.mflac',
  null,
  'test_ekey',
  60000
)
const snapInit = JSON.parse(rawInit)
assert.equal(snapInit.sessionId, sessionId)
assert.equal(snapInit.status, 'buffering')
assert.equal(snapInit.watermarkStatus, 'buffering')
assert.equal(snapInit.seekStatus, 'idle')
assert.equal(snapInit.format.encryption, 'qmc2')

// Upstream response updates total bytes
const rawResp = native.remotePlaybackShadowUpdateResponse(
  sessionId,
  206,
  10485760,
  'bytes 0-1048575/10485760',
  'audio/flac'
)
const snapResp = JSON.parse(rawResp)
assert.equal(snapResp.totalBytes, 10485760)

// Small chunk remains buffering
const rawChunk1 = native.remotePlaybackShadowReceiveChunk(sessionId, 0, 100 * 1024)
const snapChunk1 = JSON.parse(rawChunk1)
assert.equal(snapChunk1.watermarkStatus, 'buffering')
assert.equal(snapChunk1.status, 'buffering')

// Chunk reaching low watermark (256 KiB) transitions to ready
const rawChunk2 = native.remotePlaybackShadowReceiveChunk(sessionId, 100 * 1024, 200 * 1024)
const snapChunk2 = JSON.parse(rawChunk2)
assert.equal(snapChunk2.watermarkStatus, 'ready')
assert.equal(snapChunk2.status, 'ready')

// Play state
const rawPlay = native.remotePlaybackShadowSetPlayState(sessionId, true)
const snapPlay = JSON.parse(rawPlay)
assert.equal(snapPlay.status, 'playing')

// Seek: 30000 ms of 60000 ms track (50% = 5242880 bytes)
const rawSeek = native.remotePlaybackShadowSeek(sessionId, 30000, 60000)
const snapSeek = JSON.parse(rawSeek)
assert.equal(snapSeek.seekStatus, 'seeking')
assert.equal(snapSeek.seekTargetByte, 5242880)
assert.equal(snapSeek.status, 'buffering')

// Range response acknowledging seek
native.remotePlaybackShadowUpdateResponse(
  sessionId,
  206,
  5242880,
  'bytes 5242880-10485759/10485760',
  null
)

// First chunk at seek offset satisfies low watermark
const rawSeekChunk = native.remotePlaybackShadowReceiveChunk(sessionId, 5242880, 300 * 1024)
const snapSeekChunk = JSON.parse(rawSeekChunk)
assert.equal(snapSeekChunk.seekStatus, 'complete')
assert.equal(snapSeekChunk.watermarkStatus, 'ready')
assert.equal(snapSeekChunk.status, 'ready')

// Snapshot and close
const rawSnap = native.remotePlaybackShadowSnapshot(sessionId)
assert.ok(rawSnap)
assert.equal(JSON.parse(rawSnap).sessionId, sessionId)

assert.equal(native.remotePlaybackShadowClose(sessionId), true)
assert.equal(native.remotePlaybackShadowSnapshot(sessionId), null)

// 4. Verification via bridge-runtime wrappers
const runtimeFmt = nativeRemoteStreamNegotiateFormat('https://example.com/test.opus')
assert.equal(runtimeFmt.mimeType, 'audio/ogg')
assert.equal(runtimeFmt.container, 'opus')

const runtimeSession = 'm37-runtime-session'
const rInit = nativeRemotePlaybackShadowCreate(runtimeSession, 'https://example.com/test.opus', null, null, 120000)
assert.equal(rInit.sessionId, runtimeSession)
assert.equal(rInit.format.container, 'opus')

nativeRemotePlaybackShadowUpdateResponse(runtimeSession, 200, 2048576, null, null)
const rChunk = nativeRemotePlaybackShadowReceiveChunk(runtimeSession, 0, 300 * 1024)
assert.equal(rChunk.watermarkStatus, 'ready')

const rSeek = nativeRemotePlaybackShadowSeek(runtimeSession, 60000, 120000)
assert.equal(rSeek.seekStatus, 'seeking')
assert.equal(rSeek.seekTargetByte, 1024288)

assert.equal(nativeRemotePlaybackShadowClose(runtimeSession), true)
assert.equal(nativeRemotePlaybackShadowSnapshot(runtimeSession), null)

console.log('M37 Rust remote playback shadow session and format negotiation smoke test passed')
