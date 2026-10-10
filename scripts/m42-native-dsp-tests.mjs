import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import {
  nativeDspPipelineCreate,
  nativeDspPipelineConfigure,
  nativeDspPipelineProcess,
  nativeDspPipelineReset,
  nativeDspPipelineSnapshot,
  nativeDspPipelineClose
} from '../src/main/native/bridge-runtime.js'

function generateSineF32Chunk(freq = 1000, sampleRate = 48000, frameCount = 256, amplitude = 0.5) {
  // Stereo interleaved: frameCount * 2 samples * 4 bytes/f32
  const buf = Buffer.alloc(frameCount * 2 * 4)
  for (let i = 0; i < frameCount; i++) {
    const t = i / sampleRate
    const sample = Math.sin(2 * Math.PI * freq * t) * amplitude
    buf.writeFloatLE(sample, i * 8)
    buf.writeFloatLE(sample, i * 8 + 4)
  }
  return buf
}

function getRms(buffer) {
  let sum = 0
  const sampleCount = buffer.length / 4
  for (let i = 0; i < sampleCount; i++) {
    const val = buffer.readFloatLE(i * 4)
    sum += val * val
  }
  return Math.sqrt(sum / sampleCount)
}

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M42 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

// 1. Direct native export presence
assert.equal(typeof native.dspPipelineCreate, 'function')
assert.equal(typeof native.dspPipelineConfigure, 'function')
assert.equal(typeof native.dspPipelineProcess, 'function')
assert.equal(typeof native.dspPipelineReset, 'function')
assert.equal(typeof native.dspPipelineSnapshot, 'function')
assert.equal(typeof native.dspPipelineClose, 'function')

// 2. Direct N-API pipeline creation and snapshot
const pipelineId = 'm42-dsp-direct'
const initialRaw = native.dspPipelineCreate(
  pipelineId,
  48000,
  2,
  JSON.stringify({
    eqEnabled: false,
    eqGainsDb: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    replayGainDb: null,
    volume: 1.0,
    muted: false,
    limiterEnabled: true
  })
)
assert.ok(initialRaw)
const initialSnap = JSON.parse(initialRaw)
assert.equal(initialSnap.format.sampleRate, 48000)
assert.equal(initialSnap.format.channels, 2)
assert.equal(initialSnap.eqActive, false)
assert.equal(initialSnap.currentGainLinear, 1.0)
assert.equal(initialSnap.replayGainLinear, 1.0)

// 3. Audio pass-through (flat / disabled EQ)
const testSine = generateSineF32Chunk(1000, 48000, 256, 0.4)
const originalRms = getRms(testSine)
const processedChunk = native.dspPipelineProcess(pipelineId, Buffer.from(testSine))
assert.ok(processedChunk)
const passThroughRms = getRms(processedChunk)
assert.ok(Math.abs(passThroughRms - originalRms) < 1e-4, 'flat DSP must preserve RMS amplitude')

// 4. Equalizer boost & cut verification
// Boost 1kHz by +6 dB (frequency matches test sine)
const boostConfig = {
  eqEnabled: true,
  eqGainsDb: [0, 0, 0, 0, 0, 6.0, 0, 0, 0, 0], // band 5 is 1000 Hz
  replayGainDb: null,
  volume: 1.0,
  muted: false,
  limiterEnabled: true
}
const configRaw = native.dspPipelineConfigure(pipelineId, JSON.stringify(boostConfig))
assert.ok(configRaw)
const boostSnap = JSON.parse(configRaw)
assert.equal(boostSnap.eqActive, true)

// Run enough frames through the biquad filter so the transient stabilizes
for (let warm = 0; warm < 4; warm++) {
  native.dspPipelineProcess(pipelineId, Buffer.from(testSine))
}
const boostedChunk = native.dspPipelineProcess(pipelineId, Buffer.from(testSine))
const boostedRms = getRms(boostedChunk)
assert.ok(boostedRms > originalRms * 1.5, `boosted RMS (${boostedRms}) should be greater than original (${originalRms})`)

// Cut 1kHz by -12 dB
const cutConfig = {
  eqEnabled: true,
  eqGainsDb: [0, 0, 0, 0, 0, -12.0, 0, 0, 0, 0],
  replayGainDb: null,
  volume: 1.0,
  muted: false,
  limiterEnabled: true
}
native.dspPipelineConfigure(pipelineId, JSON.stringify(cutConfig))
for (let warm = 0; warm < 4; warm++) {
  native.dspPipelineProcess(pipelineId, Buffer.from(testSine))
}
const cutChunk = native.dspPipelineProcess(pipelineId, Buffer.from(testSine))
const cutRms = getRms(cutChunk)
assert.ok(cutRms < originalRms * 0.5, `cut RMS (${cutRms}) should be less than original (${originalRms})`)

// 5. Soft Limiter verification
// Send massive overload signal (amplitude 3.0)
const overloadSine = generateSineF32Chunk(440, 48000, 256, 3.0)
const limitedChunk = native.dspPipelineProcess(pipelineId, Buffer.from(overloadSine))
assert.ok(limitedChunk)
let maxPeak = 0
for (let i = 0; i < limitedChunk.length / 4; i++) {
  const absVal = Math.abs(limitedChunk.readFloatLE(i * 4))
  if (absVal > maxPeak) maxPeak = absVal
}
assert.ok(maxPeak <= 1.0, `soft limiter must clamp all output samples <= 1.0, got max peak: ${maxPeak}`)

// Reset & Close
assert.equal(native.dspPipelineReset(pipelineId), true)
assert.equal(native.dspPipelineClose(pipelineId), true)
assert.equal(native.dspPipelineSnapshot(pipelineId), null)

// 6. Bridge Runtime High-Level Wrapper Tests
const bridgeId = 'm42-bridge-pipeline'
const bridgeSnap = nativeDspPipelineCreate(bridgeId, 44100, 2, {
  eqEnabled: true,
  eqGainsDb: [1, 2, 3, 2, 1, 0, -1, -2, -3, -2],
  replayGainDb: -3.0,
  volume: 0.8,
  muted: false,
  limiterEnabled: true
})
assert.ok(bridgeSnap)
assert.equal(bridgeSnap.format.sampleRate, 44100)
assert.equal(bridgeSnap.format.channels, 2)
assert.equal(bridgeSnap.eqActive, true)

// Snapshot query
const queried = nativeDspPipelineSnapshot(bridgeId)
assert.ok(queried)
assert.equal(queried.format.sampleRate, 44100)

// Process chunk
const bridgeSine = generateSineF32Chunk(500, 44100, 128, 0.5)
const outBridge = nativeDspPipelineProcess(bridgeId, Buffer.from(bridgeSine))
assert.ok(outBridge)
assert.equal(outBridge.length, bridgeSine.length)

// Reset & Close
assert.equal(nativeDspPipelineReset(bridgeId), true)
assert.equal(nativeDspPipelineClose(bridgeId), true)
assert.equal(nativeDspPipelineSnapshot(bridgeId), null)

console.log('M42 Native Audio DSP Engine tests passed successfully.')
