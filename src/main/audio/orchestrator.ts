import { existsSync, readFileSync, statSync } from 'node:fs'
import { extname } from 'node:path'
import { randomBytes } from 'node:crypto'
import type { NativeAudioSnapshot } from '@common'
import {
  nativeAudioDevicesEnumerate,
  nativeAudioSinkClose,
  nativeAudioSinkConfigureDsp,
  nativeAudioSinkCreate,
  nativeAudioSinkPause,
  nativeAudioSinkPlay,
  nativeAudioSinkSeek,
  nativeAudioSinkSetVolume,
  nativeAudioSinkSnapshot,
  nativeAudioSinkStop,
  nativeAudioSinkSwitchDevice,
  nativeStreamCacheClose,
  nativeStreamCacheCreate,
  nativeStreamCacheSetTotalLength,
  nativeStreamCacheWriteChunk,
  type NativeAudioDeviceInfo,
  type NativeDspPipelineConfig,
  type NativeSinkSnapshot
} from '../native/bridge-runtime.js'

export interface LoadTrackOptions {
  filePath?: string
  cacheId?: string
  containerHint?: string
  mimeHint?: string
  forceVirtual?: boolean
  dspConfig?: Partial<NativeDspPipelineConfig>
}

export function createDefaultDspConfig(): NativeDspPipelineConfig {
  return {
    eqEnabled: false,
    eqGainsDb: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    replayGainDb: 0,
    volume: 1.0,
    muted: false,
    limiterEnabled: true
  }
}

/**
 * M45 全原生播放器会话调度中心 (Native Player Session Orchestrator)
 * 统一调度 StreamCache ➔ StreamDecoder ➔ NativeDspPipeline ➔ NativeAudioSink (WASAPI / Virtual)
 */
export class NativePlayerOrchestrator {
  private currentSinkId: string | null = null
  private currentCacheId: string | null = null
  private ownsCache: boolean = false
  private volume: number = 1.0
  private muted: boolean = false
  private dspConfig: NativeDspPipelineConfig = createDefaultDspConfig()
  private targetDevice: string | null = null
  private exclusiveMode: boolean = false
  private forceVirtual: boolean = false

  public constructor() {
    // 实例化音频编排器
  }

  /**
   * 加载音轨（支持本地文件路径直接读取或现有 StreamCache 挂载）
   */
  public async loadTrack(options: LoadTrackOptions): Promise<{
    ok: boolean
    sinkId?: string
    snapshot?: NativeSinkSnapshot
    error?: string
  }> {
    this.closeCurrent()

    const sinkId = `sink_${randomBytes(6).toString('hex')}`
    let cacheId = options.cacheId
    let containerHint = options.containerHint
    let ownsCache = false

    if (options.forceVirtual !== undefined) {
      this.forceVirtual = options.forceVirtual
    }

    if (options.filePath) {
      if (!existsSync(options.filePath)) {
        return { ok: false, error: `文件不存在: ${options.filePath}` }
      }
      try {
        const ext = extname(options.filePath).replace('.', '').toLowerCase()
        if (!containerHint) {
          containerHint = ext
        }
        const fileStat = statSync(options.filePath)
        const fileBuf = readFileSync(options.filePath)
        cacheId = `local_cache_${randomBytes(6).toString('hex')}`
        ownsCache = true

        const cacheCapacity = Math.max(fileStat.size, 8 * 1024 * 1024)
        nativeStreamCacheCreate(cacheId, cacheCapacity, null)
        nativeStreamCacheWriteChunk(cacheId, 0, fileBuf)
        nativeStreamCacheSetTotalLength(cacheId, fileStat.size)
      } catch (err: unknown) {
        return { ok: false, error: `读取本地音频文件失败: ${(err as Error).message}` }
      }
    }

    if (!cacheId) {
      return { ok: false, error: '未提供有效的音频源（filePath 或 cacheId）' }
    }

    if (options.dspConfig) {
      this.dspConfig = { ...this.dspConfig, ...options.dspConfig }
    }

    const snapshot = nativeAudioSinkCreate(
      sinkId,
      cacheId,
      containerHint ?? null,
      options.mimeHint ?? null,
      this.dspConfig,
      this.forceVirtual
    )

    if (!snapshot) {
      if (ownsCache && cacheId) {
        nativeStreamCacheClose(cacheId)
      }
      return { ok: false, error: '创建原生音频 Sink 失败' }
    }

    this.currentSinkId = sinkId
    this.currentCacheId = cacheId
    this.ownsCache = ownsCache

    if (this.volume !== 1.0 || this.muted) {
      nativeAudioSinkSetVolume(sinkId, this.volume, this.muted)
    }

    if (this.targetDevice !== null || this.exclusiveMode) {
      nativeAudioSinkSwitchDevice(sinkId, this.targetDevice, this.exclusiveMode)
    }

    const currentSnap = nativeAudioSinkSnapshot(sinkId) ?? snapshot
    return { ok: true, sinkId, snapshot: currentSnap }
  }

  /**
   * 启动播放
   */
  public play(): boolean {
    if (!this.currentSinkId) return false
    return nativeAudioSinkPlay(this.currentSinkId)
  }

  /**
   * 暂停播放
   */
  public pause(): boolean {
    if (!this.currentSinkId) return false
    return nativeAudioSinkPause(this.currentSinkId)
  }

  /**
   * 停止播放并清空缓冲
   */
  public stop(): boolean {
    if (!this.currentSinkId) return false
    return nativeAudioSinkStop(this.currentSinkId)
  }

  /**
   * 跳转时间戳（毫秒）
   */
  public seek(positionMs: number): boolean {
    if (!this.currentSinkId) return false
    return nativeAudioSinkSeek(this.currentSinkId, positionMs)
  }

  /**
   * 调节主音量与静音
   */
  public setVolume(volume: number, muted?: boolean): boolean {
    this.volume = Math.max(0, Math.min(1, volume))
    if (muted !== undefined) {
      this.muted = !!muted
    }
    if (!this.currentSinkId) return true
    return nativeAudioSinkSetVolume(this.currentSinkId, this.volume, this.muted)
  }

  /**
   * 动态下发 10 段 EQ 与 DSP 音效参数
   */
  public configureDsp(config: Partial<NativeDspPipelineConfig>): boolean {
    this.dspConfig = { ...this.dspConfig, ...config }
    if (!this.currentSinkId) return true
    const snap = nativeAudioSinkConfigureDsp(this.currentSinkId, this.dspConfig)
    return snap !== null
  }

  /**
   * 枚举系统所有音频输出设备
   */
  public getDevices(): NativeAudioDeviceInfo[] {
    return nativeAudioDevicesEnumerate()
  }

  /**
   * 动态热切换目标设备或开启 WASAPI 独占模式
   */
  public switchDevice(deviceName?: string | null, exclusive?: boolean): boolean {
    this.targetDevice = deviceName ?? null
    if (exclusive !== undefined) {
      this.exclusiveMode = exclusive
    }
    if (!this.currentSinkId) return true
    return nativeAudioSinkSwitchDevice(this.currentSinkId, this.targetDevice, this.exclusiveMode)
  }

  /**
   * 获取原生 Sink 快照
   */
  public getSnapshot(): NativeSinkSnapshot | null {
    if (!this.currentSinkId) return null
    return nativeAudioSinkSnapshot(this.currentSinkId)
  }

  /**
   * 合成为历史兼容的 NativeAudioSnapshot 格式，保障平滑回退
   */
  public synthesizeLegacySnapshot(): NativeAudioSnapshot | null {
    const snap = this.getSnapshot()
    if (!snap) return null
    const statusMap: Record<string, 'idle' | 'paused' | 'playing' | 'ended'> = {
      idle: 'idle',
      buffering: 'playing',
      playing: 'playing',
      paused: 'paused',
      ended: 'ended',
      error: 'idle'
    }
    return {
      status: statusMap[snap.status] ?? 'idle',
      positionMs: snap.positionMs,
      durationMs: snap.durationMs,
      format: {
        sampleRate: snap.outputFormat.sampleRate,
        channels: snap.outputFormat.channels
      },
      deviceName: snap.deviceName,
      queuedFrames: snap.queuedFrames,
      droppedFrames: Number(snap.droppedFrames)
    }
  }

  /**
   * 关闭当前会话
   */
  public closeCurrent(): void {
    if (this.currentSinkId) {
      nativeAudioSinkStop(this.currentSinkId)
      nativeAudioSinkClose(this.currentSinkId)
      this.currentSinkId = null
    }
    if (this.ownsCache && this.currentCacheId) {
      nativeStreamCacheClose(this.currentCacheId)
      this.currentCacheId = null
      this.ownsCache = false
    }
  }

  /**
   * 当前是否有活动的 Sink 会话
   */
  public isActive(): boolean {
    return this.currentSinkId !== null
  }
}

export const nativePlayerOrchestrator = new NativePlayerOrchestrator()
