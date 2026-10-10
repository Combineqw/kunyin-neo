import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  NativePlayerOrchestrator,
  createDefaultDspConfig
} from '../src/main/audio/orchestrator.ts'
import {
  nativeStreamCacheCreate,
  nativeStreamCacheWriteChunk,
  nativeStreamCacheSetTotalLength,
  nativeStreamCacheClose
} from '../src/main/native/bridge-runtime.js'

function generateSineWavBuffer(sampleRate = 44100, channels = 2, durationSec = 1.0, freq = 440) {
  const numSamples = Math.floor(sampleRate * durationSec)
  const bytesPerSample = 2
  const blockAlign = channels * bytesPerSample
  const byteRate = sampleRate * blockAlign
  const dataSize = numSamples * blockAlign
  const buffer = Buffer.alloc(44 + dataSize)

  buffer.write('RIFF', 0)
  buffer.writeUInt32LE(36 + dataSize, 4)
  buffer.write('WAVE', 8)
  buffer.write('fmt ', 12)
  buffer.writeUInt32LE(16, 16)
  buffer.writeUInt16LE(1, 20) // PCM
  buffer.writeUInt16LE(channels, 22)
  buffer.writeUInt32LE(sampleRate, 24)
  buffer.writeUInt32LE(byteRate, 28)
  buffer.writeUInt16LE(blockAlign, 32)
  buffer.writeUInt16LE(16, 34) // 16-bit
  buffer.write('data', 36)
  buffer.writeUInt32LE(dataSize, 40)

  let offset = 44
  for (let i = 0; i < numSamples; i++) {
    const t = i / sampleRate
    const sampleVal = Math.round(Math.sin(2 * Math.PI * freq * t) * 16000)
    for (let c = 0; c < channels; c++) {
      buffer.writeInt16LE(sampleVal, offset)
      offset += 2
    }
  }

  return buffer
}

async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

async function runM45Tests() {
  const testDir = path.join(path.dirname(fileURLToPath(import.meta.url)), '../temp-m45-test')
  if (!fs.existsSync(testDir)) {
    fs.mkdirSync(testDir, { recursive: true })
  }
  const testWavPath = path.join(testDir, 'test_sample.wav')
  fs.writeFileSync(testWavPath, generateSineWavBuffer(44100, 2, 1.0, 440))

  try {
    const orchestrator = new NativePlayerOrchestrator()
    assert.equal(orchestrator.isActive(), false, 'Initially orchestrator should be inactive')

    // 1. 测试错误参数处理
    const failNonExist = await orchestrator.loadTrack({ filePath: path.join(testDir, 'not_found.wav') })
    assert.equal(failNonExist.ok, false, 'Non-existent file must return ok: false')
    assert.ok(failNonExist.error.includes('文件不存在'), 'Error message must reflect missing file')

    const failNoSource = await orchestrator.loadTrack({})
    assert.equal(failNoSource.ok, false, 'Empty options must return ok: false')

    // 2. 测试本地音频文件全管线加载 (forceVirtual = true)
    const loadResult = await orchestrator.loadTrack({
      filePath: testWavPath,
      forceVirtual: true,
      dspConfig: {
        eqEnabled: true,
        eqGainsDb: [1, 2, 3, 0, -1, -2, -1, 0, 1, 2]
      }
    })
    assert.equal(loadResult.ok, true, 'Local file loading must succeed')
    assert.ok(loadResult.sinkId, 'Must return valid sinkId')
    assert.ok(loadResult.snapshot, 'Must return initial snapshot')
    assert.equal(orchestrator.isActive(), true, 'Orchestrator must be active')

    // 3. 启动播放并观察状态迁移
    const played = orchestrator.play()
    assert.equal(played, true, 'play() must return true')
    await sleep(60)

    let snap = orchestrator.getSnapshot()
    assert.ok(snap, 'Snapshot must not be null')
    assert.ok(['playing', 'ended'].includes(snap.status), `Status should be playing or ended, got: ${snap.status}`)
    assert.equal(snap.isVirtual, true, 'Sink must run in virtual endpoint')

    // 4. 暂停控制
    const paused = orchestrator.pause()
    assert.equal(paused, true, 'pause() must return true')
    snap = orchestrator.getSnapshot()
    assert.ok(['paused', 'ended'].includes(snap.status), `Status should be paused or ended, got: ${snap.status}`)

    // 5. Seek 与音量控制
    const seeked = orchestrator.seek(300)
    assert.equal(seeked, true, 'seek() must return true')

    const volResult = orchestrator.setVolume(0.8, false)
    assert.equal(volResult, true, 'setVolume() must return true')

    // 6. 动态 10 段 EQ 与 DSP 重配
    const dspReconfigured = orchestrator.configureDsp({
      eqEnabled: true,
      eqGainsDb: [-3, -2, -1, 0, 1, 2, 3, 2, 1, 0],
      preampDb: -1.5,
      replayGainDb: 2.0
    })
    assert.equal(dspReconfigured, true, 'configureDsp() must return true')

    // 7. 设备枚举与动态设备热切换 (M44 穿透验证)
    const devices = orchestrator.getDevices()
    assert.ok(Array.isArray(devices), 'Devices must be an array')

    const switched = orchestrator.switchDevice('Virtual Headphone', true)
    assert.equal(switched, true, 'switchDevice() must return true')
    snap = orchestrator.getSnapshot()
    assert.equal(snap.isExclusive, true, 'Snapshot must reflect exclusive mode toggle')

    // 8. 历史兼容 NativeAudioSnapshot 格式合成
    const legacySnapshot = orchestrator.synthesizeLegacySnapshot()
    assert.ok(legacySnapshot, 'Legacy snapshot must not be null')
    assert.ok(['idle', 'paused', 'playing', 'ended'].includes(legacySnapshot.status))
    assert.ok(typeof legacySnapshot.positionMs === 'number')
    assert.ok(legacySnapshot.format.sampleRate > 0)
    assert.ok(legacySnapshot.format.channels > 0)

    // 9. 停止与资源释放
    const stopped = orchestrator.stop()
    assert.equal(stopped, true, 'stop() must return true')
    orchestrator.closeCurrent()
    assert.equal(orchestrator.isActive(), false, 'After closeCurrent(), orchestrator must be inactive')
    assert.equal(orchestrator.getSnapshot(), null, 'Snapshot after close must be null')

    // 10. 测试基于外部 StreamCache 的流式加载
    const externalCacheId = 'm45_test_stream_cache'
    const wavBuf = fs.readFileSync(testWavPath)
    nativeStreamCacheCreate(externalCacheId, 8 * 1024 * 1024, null)
    nativeStreamCacheWriteChunk(externalCacheId, 0, wavBuf)
    nativeStreamCacheSetTotalLength(externalCacheId, wavBuf.length)

    const streamLoadResult = await orchestrator.loadTrack({
      cacheId: externalCacheId,
      containerHint: 'wav',
      forceVirtual: true
    })
    assert.equal(streamLoadResult.ok, true, 'Loading from external StreamCache must succeed')
    assert.equal(orchestrator.isActive(), true)

    assert.equal(orchestrator.play(), true)
    await sleep(40)
    assert.equal(orchestrator.stop(), true)
    orchestrator.closeCurrent()
    nativeStreamCacheClose(externalCacheId)

    console.log('M45 Native Player Session Orchestrator tests passed successfully.')
  } finally {
    if (fs.existsSync(testDir)) {
      try {
        fs.rmSync(testDir, { recursive: true, force: true })
      } catch {
        // ignore cleanup error
      }
    }
  }
}

runM45Tests().catch((err) => {
  console.error('M45 test failed:', err)
  process.exit(1)
})
