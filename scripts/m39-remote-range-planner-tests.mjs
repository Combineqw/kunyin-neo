import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import {
  nativeRemoteStreamNegotiateFormat,
  nativeRemotePlaybackShadowCreate,
  nativeRemotePlaybackShadowUpdateResponse,
  nativeRemotePlaybackShadowReceiveChunk,
  nativeRemotePlaybackShadowSeek,
  nativeRemotePlaybackShadowSnapshot,
  nativeRemotePlaybackShadowClose,
  nativeRemoteStreamPlanRange,
  nativeRemoteStreamGetBufferedRanges,
  nativeRemoteStreamIsOffsetBuffered,
  nativeRemoteStreamCalculateSeekRange
} from '../src/main/native/bridge-runtime.js'

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M39 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

// 1. Direct native export presence
assert.equal(typeof native.remoteStreamPlanRange, 'function')
assert.equal(typeof native.remoteStreamGetBufferedRanges, 'function')
assert.equal(typeof native.remoteStreamIsOffsetBuffered, 'function')
assert.equal(typeof native.remoteStreamCalculateSeekRange, 'function')

// 2. Direct N-API scheduling behavior
const sessionId = 'm39-direct-session'
native.remotePlaybackShadowCreate(
  sessionId,
  'https://music.tc.qq.com/track.mflac',
  'audio/flac',
  null,
  60000
)

// Response headers: 10,000,000 bytes (~9.5 MB)
native.remotePlaybackShadowUpdateResponse(
  sessionId,
  206,
  10000000,
  'bytes 0-9999999/10000000',
  'audio/flac'
)

// A. Initial state: empty buffer -> Critical priority
const rawPlan0 = native.remoteStreamPlanRange(sessionId, 0, 524288)
assert.ok(rawPlan0, 'plan should exist')
const plan0 = JSON.parse(rawPlan0)
assert.equal(plan0.rangeStart, 0)
assert.equal(plan0.rangeEnd, 524288)
assert.equal(plan0.priority, 'critical')
assert.equal(plan0.isCached, false)
assert.equal(plan0.httpRangeHeader, 'bytes=0-524287')

// B. Buffer 128 KiB: < low watermark (256 KiB) -> High priority
native.remotePlaybackShadowReceiveChunk(sessionId, 0, 131072)
assert.equal(native.remoteStreamIsOffsetBuffered(sessionId, 0), true)
assert.equal(native.remoteStreamIsOffsetBuffered(sessionId, 65536), true)
assert.equal(native.remoteStreamIsOffsetBuffered(sessionId, 131072), false)

const rawRanges1 = native.remoteStreamGetBufferedRanges(sessionId)
const ranges1 = JSON.parse(rawRanges1)
assert.equal(ranges1.length, 1)
assert.equal(ranges1[0].start, 0)
assert.equal(ranges1[0].end, 131072)

const rawPlan1 = native.remoteStreamPlanRange(sessionId, 0, 524288)
const plan1 = JSON.parse(rawPlan1)
assert.equal(plan1.rangeStart, 131072)
assert.equal(plan1.priority, 'high')

// C. Buffer up to 1 MiB (between low 256 KiB and high 2 MiB) -> Normal priority
native.remotePlaybackShadowReceiveChunk(sessionId, 131072, 917504)
const rawPlan2 = native.remoteStreamPlanRange(sessionId, 0, 524288)
const plan2 = JSON.parse(rawPlan2)
assert.equal(plan2.priority, 'normal')

// D. Buffer over 2.5 MiB (above high watermark) -> Idle priority
native.remotePlaybackShadowReceiveChunk(sessionId, 1048576, 2097152)
const rawPlan3 = native.remoteStreamPlanRange(sessionId, 0, 524288)
const plan3 = JSON.parse(rawPlan3)
assert.equal(plan3.priority, 'idle')

// E. Seek calculation and discontinuous ranges
const rawSeekCalc = native.remoteStreamCalculateSeekRange(sessionId, 30000, 60000, 65536)
const seekCalc = JSON.parse(rawSeekCalc)
assert.equal(seekCalc.targetMs, 30000)
assert.equal(seekCalc.estimatedByte, 5000000)
// 5000000 / 65536 = 76 -> 76 * 65536 = 4980736
assert.equal(seekCalc.alignedRangeStart, 4980736)
assert.equal(seekCalc.plannedRangeEnd, 4980736 + 524288)
assert.equal(
  seekCalc.httpRangeHeader,
  `bytes=${seekCalc.alignedRangeStart}-${seekCalc.plannedRangeEnd - 1}`
)

// Receive discontinuous chunk around seek target: [5000000, 5300000)
native.remotePlaybackShadowReceiveChunk(sessionId, 5000000, 300000)
assert.equal(native.remoteStreamIsOffsetBuffered(sessionId, 5100000), true)
assert.equal(native.remoteStreamIsOffsetBuffered(sessionId, 4000000), false)

const rawRanges2 = native.remoteStreamGetBufferedRanges(sessionId)
const ranges2 = JSON.parse(rawRanges2)
assert.equal(ranges2.length, 2)
assert.equal(ranges2[0].start, 0)
assert.equal(ranges2[1].start, 5000000)
assert.equal(ranges2[1].end, 5300000)

// Snapshot verification of extended fields
const rawSnap = native.remotePlaybackShadowSnapshot(sessionId)
const snap = JSON.parse(rawSnap)
assert.equal(snap.bufferedRangeCount, 2)
assert.ok(snap.totalBufferedBytes > 3000000)
assert.equal(snap.isCurrentCached, true)

native.remotePlaybackShadowClose(sessionId)
assert.equal(native.remoteStreamGetBufferedRanges(sessionId), null)

// 3. Verification via bridge-runtime wrappers
const runtimeId = 'm39-runtime-session'
nativeRemotePlaybackShadowCreate(runtimeId, 'https://example.com/audio.mp3', null, null, 100000)
nativeRemotePlaybackShadowUpdateResponse(
  runtimeId,
  206,
  5000000,
  'bytes 0-4999999/5000000',
  'audio/mpeg'
)

const rPlan = nativeRemoteStreamPlanRange(runtimeId, 0, 262144)
assert.ok(rPlan)
assert.equal(rPlan.rangeStart, 0)
assert.equal(rPlan.priority, 'critical')

nativeRemotePlaybackShadowReceiveChunk(runtimeId, 0, 500000)
assert.equal(nativeRemoteStreamIsOffsetBuffered(runtimeId, 250000), true)

const rRanges = nativeRemoteStreamGetBufferedRanges(runtimeId)
assert.ok(Array.isArray(rRanges))
assert.equal(rRanges.length, 1)
assert.equal(rRanges[0].end, 500000)

const rSeek = nativeRemoteStreamCalculateSeekRange(runtimeId, 50000, 100000, 65536)
assert.ok(rSeek)
assert.equal(rSeek.estimatedByte, 2500000)

nativeRemotePlaybackShadowClose(runtimeId)
assert.equal(nativeRemoteStreamGetBufferedRanges(runtimeId), null)

console.log('M39 Rust remote stream chunk preload & range planning tests passed')
