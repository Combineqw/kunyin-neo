import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import {
  nativeStreamCacheCreate,
  nativeStreamCacheWriteChunk,
  nativeStreamCacheSetTotalLength,
  nativeStreamCacheClose,
  nativeStreamDecoderCreate,
  nativeStreamDecoderProbe,
  nativeStreamDecoderStep,
  nativeStreamDecoderSeek,
  nativeStreamDecoderSnapshot,
  nativeStreamDecoderClose
} from '../src/main/native/bridge-runtime.js'

function buildWavBuffer(sampleCount = 200) {
  const dataLen = sampleCount * 2
  const riffLen = 36 + dataLen
  const buf = Buffer.alloc(44 + dataLen)

  buf.write('RIFF', 0)
  buf.writeUInt32LE(riffLen, 4)
  buf.write('WAVEfmt ', 8)
  buf.writeUInt32LE(16, 16)
  buf.writeUInt16LE(1, 20) // PCM
  buf.writeUInt16LE(2, 22) // 2 channels
  buf.writeUInt32LE(44100, 24) // sample rate
  buf.writeUInt32LE(176400, 28) // byte rate
  buf.writeUInt16LE(4, 32) // block align
  buf.writeUInt16LE(16, 34) // 16 bits
  buf.write('data', 36)
  buf.writeUInt32LE(dataLen, 40)

  for (let i = 0; i < sampleCount; i++) {
    const val = Math.round(Math.sin((i / sampleCount) * Math.PI * 4) * 16000)
    buf.writeInt16LE(val, 44 + i * 2)
  }

  return buf
}

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M41 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

// 1. Direct native export presence
assert.equal(typeof native.streamDecoderCreate, 'function')
assert.equal(typeof native.streamDecoderProbe, 'function')
assert.equal(typeof native.streamDecoderStep, 'function')
assert.equal(typeof native.streamDecoderSeek, 'function')
assert.equal(typeof native.streamDecoderSnapshot, 'function')
assert.equal(typeof native.streamDecoderClose, 'function')

// 2. Direct N-API stream decoding lifecycle
const cacheId = 'm41-direct-cache'
const decoderId = 'm41-direct-decoder'
const wavBuf = buildWavBuffer(200) // 200 samples = 100 stereo frames

// Initialize cache with total length
native.streamCacheCreate(cacheId, 512 * 1024, null)
native.streamCacheSetTotalLength(cacheId, wavBuf.length)

// Write ONLY header first (44 bytes)
native.streamCacheWriteChunk(cacheId, 0, wavBuf.subarray(0, 44))

// Create decoder
const createRaw = native.streamDecoderCreate(decoderId, cacheId, 'wav', 'audio/wav')
assert.ok(createRaw)
const createSnap = JSON.parse(createRaw)
assert.equal(createSnap.streamId, decoderId)
assert.equal(createSnap.state, 'awaitingHeader')

// Probe stream
const probeRaw = native.streamDecoderProbe(decoderId)
assert.ok(probeRaw)
const probeInfo = JSON.parse(probeRaw)
assert.equal(probeInfo.format.sampleRate, 44100)
assert.equal(probeInfo.format.channels, 2)
assert.ok(typeof probeInfo.codec === 'string' && probeInfo.codec.length > 0)

// Attempt step decoding when only header is in cache -> should report underflow
const step1Raw = native.streamDecoderStep(decoderId)
const step1 = JSON.parse(step1Raw)
assert.equal(step1.status, 'needMoreData')
assert.equal(step1.underflow, true)
assert.equal(step1.frames, 0)

// Write the remaining audio payload chunk at offset 44
native.streamCacheWriteChunk(cacheId, 44, wavBuf.subarray(44))

// Now step decoding succeeds and produces decoded PCM frames
const step2Raw = native.streamDecoderStep(decoderId)
const step2 = JSON.parse(step2Raw)
assert.equal(step2.status, 'chunk')
assert.ok(step2.frames > 0)
assert.equal(step2.underflow, false)

// Continue decoding until EndOfStream
let totalFrames = step2.frames
let reachedEof = false
for (let iter = 0; iter < 10; iter++) {
  const stepRaw = native.streamDecoderStep(decoderId)
  const step = JSON.parse(stepRaw)
  if (step.status === 'chunk') {
    totalFrames += step.frames
  }
  if (step.status === 'endOfStream' || step.endOfStream) {
    reachedEof = true
    break
  }
}

assert.ok(reachedEof, 'stream decoder should reach endOfStream')
assert.equal(totalFrames, 100, 'must decode exactly 100 frames for 200 stereo samples')

const snapEndRaw = native.streamDecoderSnapshot(decoderId)
const snapEnd = JSON.parse(snapEndRaw)
assert.equal(snapEnd.state, 'endOfStream')
assert.equal(snapEnd.decodedFrames, 100)

// Seek back to 0ms
const seekRaw = native.streamDecoderSeek(decoderId, 0)
assert.ok(seekRaw)
const seekSnap = JSON.parse(seekRaw)
assert.equal(seekSnap.state, 'ready')
assert.equal(seekSnap.positionMs, 0)

// Clean up
assert.equal(native.streamDecoderClose(decoderId), true)
assert.equal(native.streamDecoderSnapshot(decoderId), null)
assert.equal(native.streamCacheClose(cacheId), true)

// 3. Verification via bridge-runtime wrappers
const rCacheId = 'm41-runtime-cache'
const rDecoderId = 'm41-runtime-decoder'
const rWavBuf = buildWavBuffer(160) // 80 stereo frames

nativeStreamCacheCreate(rCacheId, 512 * 1024, null)
nativeStreamCacheWriteChunk(rCacheId, 0, rWavBuf)
nativeStreamCacheSetTotalLength(rCacheId, rWavBuf.length)

const rDecoder = nativeStreamDecoderCreate(rDecoderId, rCacheId, 'wav', 'audio/wav')
assert.ok(rDecoder)
assert.equal(rDecoder.streamId, rDecoderId)

const rProbe = nativeStreamDecoderProbe(rDecoderId)
assert.ok(rProbe)
assert.equal(rProbe.format.sampleRate, 44100)
assert.equal(rProbe.format.channels, 2)

const rStep = nativeStreamDecoderStep(rDecoderId)
assert.ok(rStep)
assert.equal(rStep.status, 'chunk')
assert.ok(rStep.frames > 0)

const rSnap = nativeStreamDecoderSnapshot(rDecoderId)
assert.ok(rSnap)
assert.equal(rSnap.state, 'decoding')

const rClose = nativeStreamDecoderClose(rDecoderId)
assert.equal(rClose, true)
assert.equal(nativeStreamDecoderSnapshot(rDecoderId), null)
nativeStreamCacheClose(rCacheId)

console.log('M41 native stream decoder smoke tests passed.')
