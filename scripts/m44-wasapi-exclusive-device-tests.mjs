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
  nativeAudioSinkSnapshot,
  nativeAudioSinkClose,
  nativeAudioDevicesEnumerate,
  nativeAudioSinkSwitchDevice
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
  buf.writeUInt32LE(48000, 24) // 48 kHz
  buf.writeUInt32LE(192000, 28) // byte rate
  buf.writeUInt16LE(4, 32) // block align
  buf.writeUInt16LE(16, 34) // 16 bits
  buf.write('data', 36)
  buf.writeUInt32LE(dataLen, 40)

  for (let i = 0; i < sampleCount; i++) {
    const val = Math.round(Math.sin((i / 20) * Math.PI) * 12000)
    buf.writeInt16LE(val, 44 + i * 2)
  }

  return buf
}

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M44 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

// 1. Direct native export presence
assert.equal(typeof native.audioDevicesEnumerate, 'function')
assert.equal(typeof native.audioSinkSwitchDevice, 'function')

// 2. AudioBackendCapabilities reports exclusive mode support on Windows
const capsRaw = native.audioBackendCapabilities()
assert.ok(capsRaw)
const caps = JSON.parse(capsRaw)
assert.equal(caps.supportsExclusiveOutput, true, 'Windows platform backend must report supportsExclusiveOutput: true')

// 3. Audio devices enumeration
const devicesRaw = native.audioDevicesEnumerate()
assert.ok(devicesRaw)
const devices = JSON.parse(devicesRaw)
assert.ok(Array.isArray(devices))
if (devices.length > 0) {
  const dev = devices[0]
  assert.ok(typeof dev.name === 'string')
  assert.ok(typeof dev.isDefault === 'boolean')
  assert.ok(typeof dev.supportsExclusive === 'boolean')
  assert.ok(dev.channels >= 1)
}

// High-level bridge device enumeration
const bridgeDevices = nativeAudioDevicesEnumerate()
assert.ok(Array.isArray(bridgeDevices))

// 4. Dynamic device hot-plugging & exclusive mode toggle on running sink
const cacheId = 'm44-device-cache'
const sinkId = 'm44-device-sink'
const wavBuf = buildWavBuffer(1000)

nativeStreamCacheCreate(cacheId, 512 * 1024, null)
nativeStreamCacheSetTotalLength(cacheId, wavBuf.length)
nativeStreamCacheWriteChunk(cacheId, 0, wavBuf)

const sinkSnap = nativeAudioSinkCreate(
  sinkId,
  cacheId,
  'wav',
  'audio/wav',
  null,
  true // virtual mode for deterministic isolated unit testing
)
assert.ok(sinkSnap)
assert.equal(sinkSnap.isVirtual, true)
assert.equal(sinkSnap.isExclusive, false)
assert.equal(sinkSnap.deviceName, 'virtual-aurora-sink')

// Start playback
assert.equal(nativeAudioSinkPlay(sinkId), true)
await new Promise((r) => setTimeout(r, 40))

const playingSnap = nativeAudioSinkSnapshot(sinkId)
assert.ok(playingSnap)
assert.ok(playingSnap.status === 'playing' || playingSnap.status === 'ended')

// Hot-switch to external DAC with exclusive mode enabled
assert.equal(nativeAudioSinkSwitchDevice(sinkId, 'External-USB-DAC-BitPerfect', true), true)
const dacSnap = nativeAudioSinkSnapshot(sinkId)
assert.ok(dacSnap)
assert.equal(dacSnap.deviceName, 'External-USB-DAC-BitPerfect')
assert.equal(dacSnap.isExclusive, true)
assert.ok(dacSnap.status === 'playing' || dacSnap.status === 'ended')

// Hot-switch back to system shared headphones
assert.equal(nativeAudioSinkSwitchDevice(sinkId, 'System-Headphones-Shared', false), true)
const sharedSnap = nativeAudioSinkSnapshot(sinkId)
assert.ok(sharedSnap)
assert.equal(sharedSnap.deviceName, 'System-Headphones-Shared')
assert.equal(sharedSnap.isExclusive, false)

// Clean up
assert.equal(nativeAudioSinkStop(sinkId), true)
assert.equal(nativeAudioSinkClose(sinkId), true)
nativeStreamCacheClose(cacheId)

console.log('M44 WASAPI Exclusive Mode & Dynamic Device Hotplug tests passed successfully.')
