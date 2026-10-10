import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  NativePlayerOrchestrator
} from '../src/main/audio/orchestrator.ts'

function generateSineWavBuffer(sampleRate = 44100, channels = 2, durationSec = 0.5, freq = 440) {
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

async function runM46Tests() {
  const testDir = path.join(path.dirname(fileURLToPath(import.meta.url)), '../temp-m46-test')
  if (!fs.existsSync(testDir)) {
    fs.mkdirSync(testDir, { recursive: true })
  }
  const testWavPath = path.join(testDir, 'sync_sample.wav')
  fs.writeFileSync(testWavPath, generateSineWavBuffer(44100, 2, 0.5, 440))

  try {
    // 1. 验证 Preload API 与类型契约完备性
    const ipcFile = fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), '../src/common/types/ipc.ts'), 'utf-8')
    assert.ok(ipcFile.includes('NATIVE_PLAYER_LOAD'), 'IpcChannels must contain NATIVE_PLAYER_LOAD')
    assert.ok(ipcFile.includes('NATIVE_PLAYER_SWITCH_DEVICE'), 'IpcChannels must contain NATIVE_PLAYER_SWITCH_DEVICE')
    assert.ok(ipcFile.includes('nativePlayer: {'), 'WindowApi must contain nativePlayer definition')

    const preloadFile = fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), '../src/preload/index.ts'), 'utf-8')
    assert.ok(preloadFile.includes('nativePlayer: {'), 'Preload must expose nativePlayer')
    assert.ok(preloadFile.includes('switchDevice:'), 'Preload nativePlayer must expose switchDevice')

    // 2. 验证渲染层 Pinia Player Store 原生状态接管声明
    const storeFile = fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), '../src/renderer/src/stores/player.ts'), 'utf-8')
    assert.ok(storeFile.includes('isNativeActive'), 'Player store must export isNativeActive')
    assert.ok(storeFile.includes('switchAudioDevice'), 'Player store must export switchAudioDevice')
    assert.ok(storeFile.includes('getAudioDevices'), 'Player store must export getAudioDevices')
    assert.ok(storeFile.includes('window.api.nativePlayer.play()'), 'Player store toggle must invoke nativePlayer.play()')
    assert.ok(storeFile.includes('window.api.nativePlayer.pause()'), 'Player store toggle must invoke nativePlayer.pause()')
    assert.ok(storeFile.includes('window.api.nativePlayer.seek('), 'Player store seek must invoke nativePlayer.seek()')
    assert.ok(storeFile.includes('window.api.nativePlayer.setVolume('), 'Player store applyAudioVolume must invoke nativePlayer.setVolume()')
    assert.ok(storeFile.includes('window.api.nativePlayer\n        .configureDsp('), 'Player store applyAudioSettings must invoke nativePlayer.configureDsp()')

    // 3. 模拟渲染层 ➔ 主进程全双工调度联动
    const orchestrator = new NativePlayerOrchestrator()
    const loadResult = await orchestrator.loadTrack({
      filePath: testWavPath,
      forceVirtual: true
    })
    assert.equal(loadResult.ok, true, 'Track load must succeed')
    assert.equal(orchestrator.isActive(), true, 'Orchestrator must be active')

    // 模拟 UI 点击播放
    const playOk = orchestrator.play()
    assert.equal(playOk, true)
    await sleep(40)

    let snap = orchestrator.getSnapshot()
    assert.ok(snap)
    assert.ok(['playing', 'ended'].includes(snap.status))

    // 模拟 UI 调整音量 (0.6, false)
    orchestrator.setVolume(0.6, false)
    snap = orchestrator.getSnapshot()
    assert.ok(snap)

    // 模拟 UI 调整 10 段 EQ
    const testEqGains = [2, 3, 1, 0, -1, -2, -3, 0, 1, 2]
    orchestrator.configureDsp({
      eqEnabled: true,
      eqGainsDb: testEqGains
    })
    snap = orchestrator.getSnapshot()
    assert.ok(snap)
    assert.equal(snap.dsp.config.eqEnabled, true)
    assert.deepEqual(snap.dsp.config.eqGainsDb, testEqGains)

    // 模拟 UI 切换输出设备与独占模式
    orchestrator.switchDevice('Mastering DAC', true)
    snap = orchestrator.getSnapshot()
    assert.ok(snap)
    assert.equal(snap.isExclusive, true)
    assert.equal(snap.deviceName, 'Mastering DAC')

    // 模拟 UI 拖动进度条
    orchestrator.seek(200)
    snap = orchestrator.getSnapshot()
    assert.ok(snap)

    // 模拟 UI 暂停与恢复
    orchestrator.pause()
    snap = orchestrator.getSnapshot()
    assert.ok(['paused', 'ended'].includes(snap.status))

    orchestrator.play()
    snap = orchestrator.getSnapshot()
    assert.ok(['playing', 'ended'].includes(snap.status))

    // 销毁并确认状态清理
    orchestrator.stop()
    orchestrator.closeCurrent()
    assert.equal(orchestrator.isActive(), false)

    console.log('M46 Renderer Native Player UI Takeover & Bidirectional Sync tests passed successfully.')
  } finally {
    if (fs.existsSync(testDir)) {
      try {
        fs.rmSync(testDir, { recursive: true, force: true })
      } catch {
        // ignore
      }
    }
  }
}

runM46Tests().catch((err) => {
  console.error('M46 test failed:', err)
  process.exit(1)
})
