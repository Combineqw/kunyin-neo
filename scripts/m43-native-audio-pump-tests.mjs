import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import {
  nativeStreamCacheCreate,
  nativeStreamCacheWriteChunk,
  nativeStreamCacheSetTotalLength,
  nativeStreamCacheClose,
  nativeAudioSinkCreate,
  nativeAudioSinkPlay,
  nativeAudioSinkPause,
  nativeAudioSinkStop,
  nativeAudioSinkSeek,
  nativeAudioSinkSetVolume,
  nativeAudioSinkConfigureDsp,
  nativeAudioSinkSnapshot,
  nativeAudioSinkClose
} from '../src/main/native/bridge-runtime.js'

function buildWavBuffer(sampleCount = 1000) {
  const dataLen = sampleCount * 2
  const riffLen = 36 + dataLen
  const buf = Buffer.alloc(44 + dataLen)

  buf.write('RIFF', 0)
  buf.writeUInt32LE(riffLen, 4)
  buf.write('WAVEfmt ', 8)
  buf.writeUInt32LE(16, 16)
  buf.writeUInt16LE(1, 20) // PCM
  buf.writeUInt16LE(2, 22) // stereo
  buf.writeUInt32LE(44100, 24) // 44.1 kHz
  buf.writeUInt32LE(176400, 28) // byte rate
  buf.writeUInt16LE(4, 32) // block align
  buf.writeUInt16LE(16, 34) // 16 bits
  buf.write('data', 36)
  buf.writeUInt32LE(dataLen, 40)

  for (let i = 0; i < sampleCount; i++) {
    const val = Math.round(Math.sin((i / 25) * Math.PI) * 14000)
    buf.writeInt16LE(val, 44 + i * 2)
  }

  return buf
}

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M43 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

// 1. Direct native export presence
assert.equal(typeof native.audioSinkCreate, 'function')
assert.equal(typeof native.audioSinkPlay, 'function')
assert.equal(typeof native.audioSinkPause, 'function')
assert.equal(typeof native.audioSinkStop, 'function')
assert.equal(typeof native.audioSinkSeek, 'function')
assert.equal(typeof native.audioSinkSetVolume, 'function')
assert.equal(typeof native.audioSinkConfigureDsp, 'function')
assert.equal(typeof native.audioSinkSnapshot, 'function')
assert.equal(typeof native.audioSinkClose, 'function')

// 2. Direct N-API pipeline tests (StreamCache -> StreamDecoder -> Resampler -> Native DSP -> Sink)
const directCacheId = 'm43-direct-cache'
const directSinkId = 'm43-direct-sink'
const wavBuf = buildWavBuffer(1200)

native.streamCacheCreate(directCacheId, 1024 * 1024, null)
native.streamCacheSetTotalLength(directCacheId, wavBuf.length)
native.streamCacheWriteChunk(directCacheId, 0, wavBuf)

const directRaw = native.audioSinkCreate(
  directSinkId,
  directCacheId,
  'wav',
  'audio/wav',
  null,
  true // forceVirtual for deterministic unit test
)
assert.ok(directRaw)
const snap0 = JSON.parse(directRaw)
assert.equal(snap0.sinkId, directSinkId)
assert.equal(snap0.status, 'idle')
assert.equal(snap0.isVirtual, true)
assert.equal(snap0.outputFormat.sampleRate, 48000)

// Play
assert.equal(native.audioSinkPlay(directSinkId), true)

// Wait for pump thread to decode & push PCM frames
await new Promise((r) => setTimeout(r, 60))

const snap1Raw = native.audioSinkSnapshot(directSinkId)
assert.ok(snap1Raw)
const snap1 = JSON.parse(snap1Raw)
assert.ok(snap1.status === 'playing' || snap1.status === 'ended')
assert.ok(snap1.queuedFrames > 0 || snap1.status === 'ended')

// Configure DSP dynamically
const dspConfig = {
  eqEnabled: true,
  eqGainsDb: [0, 1, 2, 3, 2, 1, 0, -1, -2, -3],
  replayGainDb: -2.0,
  volume: 0.85,
  muted: false,
  limiterEnabled: true
}
const snapDspRaw = native.audioSinkConfigureDsp(directSinkId, JSON.stringify(dspConfig))
assert.ok(snapDspRaw)
const snapDsp = JSON.parse(snapDspRaw)
assert.equal(snapDsp.dsp.eqActive, true)
assert.equal(snapDsp.dsp.config.eqEnabled, true)

// Volume
assert.equal(native.audioSinkSetVolume(directSinkId, 0.7, false), true)

// Pause
assert.equal(native.audioSinkPause(directSinkId), true)
const snapPause = JSON.parse(native.audioSinkSnapshot(directSinkId))
assert.equal(snapPause.status, 'paused')

// Seek
assert.equal(native.audioSinkSeek(directSinkId, 0), true)
const snapSeek = JSON.parse(native.audioSinkSnapshot(directSinkId))
assert.equal(snapSeek.positionMs, 0)

// Stop
assert.equal(native.audioSinkStop(directSinkId), true)
const snapStop = JSON.parse(native.audioSinkSnapshot(directSinkId))
assert.equal(snapStop.status, 'idle')

// Close
assert.equal(native.audioSinkClose(directSinkId), true)
assert.equal(native.audioSinkSnapshot(directSinkId), null)
native.streamCacheClose(directCacheId)

// 3. High-level Bridge Runtime tests
const bridgeCacheId = 'm43-bridge-cache'
const bridgeSinkId = 'm43-bridge-sink'

nativeStreamCacheCreate(bridgeCacheId, 1024 * 1024, null)
nativeStreamCacheSetTotalLength(bridgeCacheId, wavBuf.length)
nativeStreamCacheWriteChunk(bridgeCacheId, 0, wavBuf)

const bridgeSnap = nativeAudioSinkCreate(
  bridgeSinkId,
  bridgeCacheId,
  'wav',
  'audio/wav',
  {
    eqEnabled: true,
    eqGainsDb: [0, 0, 0, 0, 0, 3, 0, 0, 0, 0],
    volume: 0.9,
    muted: false,
    limiterEnabled: true
  },
  true
)
assert.ok(bridgeSnap)
assert.equal(bridgeSnap.sinkId, bridgeSinkId)
assert.equal(bridgeSnap.status, 'idle')

assert.equal(nativeAudioSinkPlay(bridgeSinkId), true)
await new Promise((r) => setTimeout(r, 60))

const queried = nativeAudioSinkSnapshot(bridgeSinkId)
assert.ok(queried)
assert.ok(queried.status === 'playing' || queried.status === 'ended')

assert.equal(nativeAudioSinkPause(bridgeSinkId), true)
assert.equal(nativeAudioSinkSetVolume(bridgeSinkId, 0.5, false), true)
assert.equal(nativeAudioSinkSeek(bridgeSinkId, 10), true)
assert.equal(nativeAudioSinkStop(bridgeSinkId), true)
assert.equal(nativeAudioSinkClose(bridgeSinkId), true)
assert.equal(nativeAudioSinkSnapshot(bridgeSinkId), null)
nativeStreamCacheClose(bridgeCacheId)

console.log('M43 Native Audio Sink & WASAPI Direct Pump tests passed successfully.')
