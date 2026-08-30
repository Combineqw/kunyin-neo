/**
 * 渲染层唯一的有状态 Web Audio 图管理器。
 * 负责将同一个 HTMLMediaElement 接入动态 EQ、SRS 风格处理、淡入淡出和 IRS 卷积；
 * 格式解析与预设校验保持在 common/domain，Pinia store 只负责把持久化设置投递到本控制器。
 */
import type { EqualizerFilter } from '@common'

export const EQ_FREQUENCIES = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const

export const EQ_GAIN_MIN = -12
export const EQ_GAIN_MAX = 12
export const DEFAULT_FADE_DURATION_MS = 300

export const EQUALIZER_PRESETS = {
  flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  bass: [5, 4, 3, 2, 1, 0, 0, 0, 0, 0],
  vocal: [-2, -1, 0, 2, 4, 4, 3, 2, 1, 0],
  treble: [0, 0, 0, 0, 0, 1, 2, 3, 4, 5]
} as const

export interface SrsAudioSettings {
  enabled: boolean
  intensity: number
  bass: number
  voice: number
  treble: number
  space: number
  limiter: boolean
}

export interface IrsAudioSettings {
  enabled: boolean
  /** 湿声比例 0..100；干声由 100 - wetPercent 得出。 */
  wetPercent: number
}

export interface AudioGraphState {
  contextState: AudioContextState
  equalizerEnabled: boolean
  bandGains: number[]
  preampDb: number
  filters: EqualizerFilter[]
  srsEnabled: boolean
  srsIntensity: number
  srsBass: number
  srsVoice: number
  srsTreble: number
  srsSpace: number
  srsLimiter: boolean
  irsEnabled: boolean
  irsWetPercent: number
  irsDryPercent: number
  irsBufferDurationMs: number
  outputGain: number
}

/**
 * 播放 store 使用的音频图门面。
 * 该接口隔离 Web Audio 节点细节，避免业务状态层持有可重连的底层节点。
 */
export interface AudioGraphController {
  readonly analyser: AnalyserNode
  resume(): Promise<void>
  setEqualizerEnabled(enabled: boolean): void
  setBandGain(index: number, gain: number): void
  setBandGains(gains: readonly number[]): void
  setEqualizerProfile(preampDb: number | undefined, filters: readonly EqualizerFilter[]): void
  setSrsSettings(settings: SrsAudioSettings): void
  setIrsSettings(settings: IrsAudioSettings): void
  decodeIrs(data: ArrayBuffer): Promise<AudioBuffer>
  setIrsBuffer(buffer: AudioBuffer | null): void
  hasIrsBuffer(): boolean
  previewEffect(durationMs?: number): Promise<void>
  getIrsResponseCurve(pointCount?: number): number[]
  fadeIn(durationMs?: number): Promise<void>
  fadeOut(durationMs?: number): Promise<void>
  prepareFadeIn(): void
  restoreOutput(): void
  silenceImmediately(): void
  getState(): AudioGraphState
}

type GraphRegistryValue = WebAudioGraph | Error

type AudioGraphGlobal = typeof globalThis & {
  __kunyinAudioGraphs?: WeakMap<HTMLMediaElement, GraphRegistryValue>
}

const audioGraphGlobal = globalThis as AudioGraphGlobal
const graphByElement =
  audioGraphGlobal.__kunyinAudioGraphs ?? new WeakMap<HTMLMediaElement, GraphRegistryValue>()
audioGraphGlobal.__kunyinAudioGraphs = graphByElement

function clampGain(gain: number): number {
  const finiteGain = Number.isFinite(gain) ? gain : 0
  return Math.max(EQ_GAIN_MIN, Math.min(EQ_GAIN_MAX, finiteGain))
}

function clampPreamp(gain: number | undefined): number {
  return Math.max(-24, Math.min(24, Number.isFinite(gain) ? (gain as number) : 0))
}

function dbToLinearGain(db: number): number {
  return 10 ** (db / 20)
}

function clampPercent(value: number): number {
  return Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0))
}

function normalizeSrsSettings(settings: SrsAudioSettings): SrsAudioSettings {
  return {
    enabled: settings.enabled === true,
    intensity: clampPercent(settings.intensity),
    bass: clampPercent(settings.bass),
    voice: clampPercent(settings.voice),
    treble: clampPercent(settings.treble),
    space: clampPercent(settings.space),
    limiter: settings.limiter !== false
  }
}

function normalizeFilter(filter: EqualizerFilter): EqualizerFilter {
  return {
    type: filter.type,
    frequency: Math.max(1, Math.min(24000, Number.isFinite(filter.frequency) ? filter.frequency : 1000)),
    gainDb: Math.max(-24, Math.min(24, Number.isFinite(filter.gainDb) ? filter.gainDb : 0)),
    ...(filter.q && filter.q > 0 ? { q: Math.min(100, filter.q) } : {}),
    ...(filter.enabled === false ? { enabled: false } : {})
  }
}

function filtersFromBands(gains: readonly number[]): EqualizerFilter[] {
  return EQ_FREQUENCIES.map((frequency, index) => ({
    type: index === 0 ? 'lowshelf' : index === EQ_FREQUENCIES.length - 1 ? 'highshelf' : 'peaking',
    frequency,
    gainDb: clampGain(gains[index] ?? 0),
    q: index === 0 || index === EQ_FREQUENCIES.length - 1 ? 0.7 : 1.4
  }))
}

/**
 * 单个 HTMLMediaElement 的 Web Audio 节点图实现。
 * 实例只能经 getOrCreateAudioGraph 创建，以遵守 createMediaElementSource 的单元素单次调用限制。
 */
class WebAudioGraph implements AudioGraphController {
  readonly analyser: AnalyserNode

  private readonly context: AudioContext
  private readonly source: MediaElementAudioSourceNode
  private readonly previewInput: GainNode
  private readonly preamp: GainNode
  private filters: BiquadFilterNode[] = []
  private readonly srsBass: BiquadFilterNode
  private readonly srsVoice: BiquadFilterNode
  private readonly srsTreble: BiquadFilterNode
  private readonly srsSplitter: ChannelSplitterNode
  private readonly srsMerger: ChannelMergerNode
  private readonly srsMidLeft: GainNode
  private readonly srsMidRight: GainNode
  private readonly srsSideLeft: GainNode
  private readonly srsSideRight: GainNode
  private readonly srsLimiter: DynamicsCompressorNode
  private readonly srsOutput: GainNode
  private readonly irsDry: GainNode
  private readonly irsConvolver: ConvolverNode
  private readonly irsWet: GainNode
  private readonly irsMerger: GainNode
  private readonly output: GainNode
  private readonly bandGains = EQ_FREQUENCIES.map(() => 0)
  private profileFilters: EqualizerFilter[] = filtersFromBands(this.bandGains)
  private preampDb = 0
  private equalizerEnabled = true
  private srs: SrsAudioSettings = {
    enabled: false,
    intensity: 100,
    bass: 35,
    voice: 20,
    treble: 18,
    space: 24,
    limiter: true
  }
  private irs: IrsAudioSettings = { enabled: false, wetPercent: 50 }
  private irsBuffer: AudioBuffer | null = null

  constructor(element: HTMLMediaElement) {
    this.context = new AudioContext()
    this.source = this.context.createMediaElementSource(element)
    this.previewInput = this.context.createGain()
    this.preamp = this.context.createGain()
    this.srsBass = this.context.createBiquadFilter()
    this.srsBass.type = 'lowshelf'
    this.srsBass.frequency.value = 95
    this.srsVoice = this.context.createBiquadFilter()
    this.srsVoice.type = 'peaking'
    this.srsVoice.frequency.value = 2500
    this.srsVoice.Q.value = 0.85
    this.srsTreble = this.context.createBiquadFilter()
    this.srsTreble.type = 'highshelf'
    this.srsTreble.frequency.value = 10000
    this.srsSplitter = this.context.createChannelSplitter(2)
    this.srsMerger = this.context.createChannelMerger(2)
    this.srsMidLeft = this.context.createGain()
    this.srsMidRight = this.context.createGain()
    this.srsSideLeft = this.context.createGain()
    this.srsSideRight = this.context.createGain()
    this.srsLimiter = this.context.createDynamicsCompressor()
    this.srsLimiter.knee.value = 12
    this.srsLimiter.attack.value = 0.003
    this.srsLimiter.release.value = 0.12
    this.srsOutput = this.context.createGain()
    this.irsDry = this.context.createGain()
    this.irsConvolver = this.context.createConvolver()
    this.irsWet = this.context.createGain()
    this.irsMerger = this.context.createGain()
    this.output = this.context.createGain()
    this.output.gain.value = 1
    this.analyser = this.context.createAnalyser()
    this.analyser.fftSize = 256
    this.analyser.smoothingTimeConstant = 0.76
    this.analyser.minDecibels = -90
    this.analyser.maxDecibels = -18
    this.rebuildFilterChain()
    // output 是唯一淡入淡出闸门；分析器置于所有处理之后，频谱才能反映最终听到的信号。
    this.output.connect(this.analyser)
    this.analyser.connect(this.context.destination)
  }

  async resume(): Promise<void> {
    if (this.context.state === 'suspended') await this.context.resume()
    if (this.context.state !== 'running') throw new Error(`AudioContext 未运行：${this.context.state}`)
  }

  setEqualizerEnabled(enabled: boolean): void {
    this.equalizerEnabled = enabled
    this.applyEqualizer()
  }

  setBandGain(index: number, gain: number): void {
    if (!Number.isInteger(index) || index < 0 || index >= this.bandGains.length) return
    this.bandGains[index] = clampGain(gain)
    this.profileFilters = filtersFromBands(this.bandGains)
    this.rebuildFilterChain()
  }

  setBandGains(gains: readonly number[]): void {
    for (let index = 0; index < this.bandGains.length; index++) {
      this.bandGains[index] = clampGain(gains[index] ?? 0)
    }
    this.profileFilters = filtersFromBands(this.bandGains)
    this.rebuildFilterChain()
  }

  /**
   * 应用包含 Preamp 与动态滤镜描述的 EQ 预设。
   * @param preampDb 前置增益分贝值。
   * @param filters 一条描述对应一个 BiquadFilterNode 的动态滤镜链。
   */
  setEqualizerProfile(preampDb: number | undefined, filters: readonly EqualizerFilter[]): void {
    this.preampDb = clampPreamp(preampDb)
    this.profileFilters = filters.length
      ? filters.map(normalizeFilter)
      : filtersFromBands(this.bandGains)
    this.rebuildFilterChain()
  }

  setSrsSettings(settings: SrsAudioSettings): void {
    this.srs = normalizeSrsSettings(settings)
    this.applySrs()
  }

  setIrsSettings(settings: IrsAudioSettings): void {
    this.irs = {
      enabled: settings.enabled === true,
      wetPercent: clampPercent(settings.wetPercent)
    }
    this.applyIrs()
  }

  /**
   * 将已由主进程校验的 IRS 字节解码为 Web Audio 可复用的 AudioBuffer。
   * @param data IRS 文件的 ArrayBuffer 副本。
   * @returns 可直接赋给 ConvolverNode 的解码结果。
   */
  decodeIrs(data: ArrayBuffer): Promise<AudioBuffer> {
    return this.context.decodeAudioData(data.slice(0))
  }

  setIrsBuffer(buffer: AudioBuffer | null): void {
    this.irsBuffer = buffer
    this.irsConvolver.buffer = buffer
    this.applyIrs()
  }

  hasIrsBuffer(): boolean {
    return !!this.irsBuffer
  }

  async previewEffect(durationMs = 1200): Promise<void> {
    await this.resume()
    const length = Math.max(0.2, Math.min(4, durationMs / 1000))
    const sampleRate = this.context.sampleRate
    const buffer = this.context.createBuffer(2, Math.floor(sampleRate * length), sampleRate)
    for (let channel = 0; channel < buffer.numberOfChannels; channel++) {
      const data = buffer.getChannelData(channel)
      for (let index = 0; index < data.length; index++) {
        const progress = index / Math.max(1, data.length - 1)
        const frequency = 180 + progress * 820
        const envelope = Math.min(1, progress * 18, (1 - progress) * 18)
        data[index] = Math.sin(2 * Math.PI * frequency * (index / sampleRate)) * envelope * 0.22
      }
    }
    const node = this.context.createBufferSource()
    node.buffer = buffer
    node.connect(this.previewInput)
    node.start()
    node.stop(this.context.currentTime + length)
    await new Promise<void>((resolve) => {
      node.addEventListener('ended', () => resolve(), { once: true })
    })
  }

  getIrsResponseCurve(pointCount = 48): number[] {
    const buffer = this.irsBuffer
    if (!buffer) return []
    const count = Math.max(8, Math.min(96, Math.floor(pointCount)))
    const samples = Math.min(2048, buffer.length)
    const channel = buffer.getChannelData(0)
    const result: number[] = []
    for (let point = 0; point < count; point++) {
      const frequency = 20 * (20000 / 20) ** (point / Math.max(1, count - 1))
      const bin = Math.max(1, Math.min(Math.floor((frequency / buffer.sampleRate) * samples), Math.floor(samples / 2)))
      let real = 0
      let imaginary = 0
      for (let index = 0; index < samples; index++) {
        const phase = (2 * Math.PI * bin * index) / samples
        const window = 0.5 - 0.5 * Math.cos((2 * Math.PI * index) / Math.max(1, samples - 1))
        real += channel[index] * window * Math.cos(phase)
        imaginary -= channel[index] * window * Math.sin(phase)
      }
      const magnitude = Math.sqrt(real * real + imaginary * imaginary) / Math.max(1, samples)
      result.push(Math.max(0, Math.min(1, (20 * Math.log10(Math.max(magnitude, 0.00001)) + 60) / 60)))
    }
    return result
  }

  fadeIn(durationMs = DEFAULT_FADE_DURATION_MS): Promise<void> {
    return this.rampOutput(1, durationMs)
  }

  fadeOut(durationMs = DEFAULT_FADE_DURATION_MS): Promise<void> {
    return this.rampOutput(0, durationMs)
  }

  prepareFadeIn(): void {
    this.setOutputImmediately(0)
  }

  restoreOutput(): void {
    this.setOutputImmediately(1)
  }

  silenceImmediately(): void {
    this.setOutputImmediately(0)
  }

  getState(): AudioGraphState {
    return {
      contextState: this.context.state,
      equalizerEnabled: this.equalizerEnabled,
      bandGains: [...this.bandGains],
      preampDb: this.preampDb,
      filters: this.profileFilters.map((filter) => ({ ...filter })),
      srsEnabled: this.srs.enabled,
      srsIntensity: this.srs.intensity,
      srsBass: this.srs.bass,
      srsVoice: this.srs.voice,
      srsTreble: this.srs.treble,
      srsSpace: this.srs.space,
      srsLimiter: this.srs.limiter,
      irsEnabled: this.irs.enabled,
      irsWetPercent: this.irs.wetPercent,
      irsDryPercent: 100 - this.irs.wetPercent,
      irsBufferDurationMs: this.irsBuffer ? this.irsBuffer.duration * 1000 : 0,
      outputGain: this.output.gain.value
    }
  }

  /**
   * 重新连接动态 EQ 之后的固定处理链。
   * 新预设的 Filter 数量不固定，必须先断开旧链再按描述重建，避免旧节点残留并行输出。
   */
  private rebuildFilterChain(): void {
    this.source.disconnect()
    this.previewInput.disconnect()
    this.preamp.disconnect()
    for (const filter of this.filters) filter.disconnect()
    this.srsBass.disconnect()
    this.srsVoice.disconnect()
    this.srsTreble.disconnect()
    this.srsSplitter.disconnect()
    this.srsMerger.disconnect()
    this.srsMidLeft.disconnect()
    this.srsMidRight.disconnect()
    this.srsSideLeft.disconnect()
    this.srsSideRight.disconnect()
    this.srsLimiter.disconnect()
    this.srsOutput.disconnect()
    this.irsDry.disconnect()
    this.irsConvolver.disconnect()
    this.irsWet.disconnect()
    this.irsMerger.disconnect()
    this.filters = this.profileFilters.map((description) => {
      const filter = this.context.createBiquadFilter()
      filter.type = description.type
      filter.frequency.value = description.frequency
      filter.Q.value = description.q ?? (description.type === 'peaking' ? 1.4 : 0.7)
      filter.gain.value = description.enabled === false ? 0 : description.gainDb
      return filter
    })
    this.source.connect(this.preamp)
    this.previewInput.connect(this.preamp)
    let tail: AudioNode = this.preamp
    tail = this.preamp
    for (const filter of this.filters) {
      tail.connect(filter)
      tail = filter
    }
    tail.connect(this.srsBass)
    this.srsBass.connect(this.srsVoice)
    this.srsVoice.connect(this.srsTreble)
    this.srsTreble.connect(this.srsSplitter)

    // Mid/Side 空间扩展：保留中心信息，按宽度比例混入反相侧声道。
    // 四个增益节点只做矩阵系数，不直接放大总电平，关闭时严格还原 L/R。
    this.srsSplitter.connect(this.srsMidLeft, 0)
    this.srsSplitter.connect(this.srsMidRight, 1)
    this.srsSplitter.connect(this.srsSideLeft, 1)
    this.srsSplitter.connect(this.srsSideRight, 0)
    this.srsMidLeft.connect(this.srsMerger, 0, 0)
    this.srsMidRight.connect(this.srsMerger, 0, 1)
    this.srsSideLeft.connect(this.srsMerger, 0, 0)
    this.srsSideRight.connect(this.srsMerger, 0, 1)
    this.srsMerger.connect(this.srsLimiter)
    this.srsLimiter.connect(this.srsOutput)
    // IRS 放在淡入淡出 output 之前：切歌渐变同时覆盖干声和卷积尾音，防止湿声残留突变。
    // 两路并联而非串联，干湿比例才能在不重解码卷积 buffer 的情况下实时调整。
    this.srsOutput.connect(this.irsDry)
    this.srsOutput.connect(this.irsConvolver)
    this.irsConvolver.connect(this.irsWet)
    this.irsDry.connect(this.irsMerger)
    this.irsWet.connect(this.irsMerger)
    this.irsMerger.connect(this.output)
    this.applyEqualizer()
    this.applySrs()
    this.irsConvolver.buffer = this.irsBuffer
    this.applyIrs()
  }

  private applyEqualizer(): void {
    const now = this.context.currentTime
    this.preamp.gain.setTargetAtTime(
      this.equalizerEnabled ? dbToLinearGain(this.preampDb) : 1,
      now,
      0.015
    )
    for (let index = 0; index < this.filters.length; index++) {
      const description = this.profileFilters[index]
      const value = this.equalizerEnabled && description?.enabled !== false ? description?.gainDb ?? 0 : 0
      this.filters[index].gain.setTargetAtTime(value, now, 0.015)
    }
  }

  private applySrs(): void {
    const now = this.context.currentTime
    const srs = this.srs
    const enabled = srs.enabled
    const intensity = srs.intensity / 100
    const bassDb = enabled ? (srs.bass / 100) * 5 * intensity : 0
    const voiceDb = enabled ? (srs.voice / 100) * 3.5 * intensity : 0
    const trebleDb = enabled ? (srs.treble / 100) * 3 * intensity : 0
    const width = enabled ? (srs.space / 100) * 0.45 * intensity : 0

    this.srsBass.gain.setTargetAtTime(bassDb, now, 0.02)
    this.srsVoice.gain.setTargetAtTime(voiceDb, now, 0.02)
    this.srsTreble.gain.setTargetAtTime(trebleDb, now, 0.02)
    this.srsMidLeft.gain.setTargetAtTime(1 + width, now, 0.02)
    this.srsMidRight.gain.setTargetAtTime(1 + width, now, 0.02)
    this.srsSideLeft.gain.setTargetAtTime(-width, now, 0.02)
    this.srsSideRight.gain.setTargetAtTime(-width, now, 0.02)
    this.srsLimiter.threshold.setTargetAtTime(enabled && srs.limiter ? -3 : 0, now, 0.02)
    this.srsLimiter.ratio.setTargetAtTime(enabled && srs.limiter ? 8 : 1, now, 0.02)
    this.srsLimiter.knee.setTargetAtTime(enabled && srs.limiter ? 12 : 0, now, 0.02)
    this.srsOutput.gain.setTargetAtTime(enabled ? 1 / (1 + width * 0.72) : 1, now, 0.02)
  }

  /**
   * 调整 IRS 干湿混合增益。
   * 关闭时保留 ConvolverNode.buffer，仅将湿声归零；切歌或再次打开时无需重新解码预设。
   */
  private applyIrs(): void {
    const now = this.context.currentTime
    const active = this.irs.enabled && !!this.irsBuffer
    const wet = active ? this.irs.wetPercent / 100 : 0
    const dry = active ? 1 - wet : 1
    this.irsDry.gain.setTargetAtTime(dry, now, 0.02)
    this.irsWet.gain.setTargetAtTime(wet, now, 0.02)
  }

  private holdOutput(now: number): void {
    if (typeof this.output.gain.cancelAndHoldAtTime === 'function') {
      this.output.gain.cancelAndHoldAtTime(now)
      return
    }
    const current = this.output.gain.value
    this.output.gain.cancelScheduledValues(now)
    this.output.gain.setValueAtTime(current, now)
  }

  private setOutputImmediately(value: number): void {
    const now = this.context.currentTime
    this.holdOutput(now)
    this.output.gain.setValueAtTime(value, now)
  }

  private async rampOutput(target: number, durationMs: number): Promise<void> {
    await this.resume()
    const duration = Math.max(0, durationMs) / 1000
    const now = this.context.currentTime
    this.holdOutput(now)
    this.output.gain.linearRampToValueAtTime(target, now + duration)
    if (durationMs > 0) await new Promise<void>((resolve) => setTimeout(resolve, durationMs))
  }
}

/** 同一个媒体元素只能创建一个 MediaElementAudioSourceNode。 */
export function getOrCreateAudioGraph(element: HTMLMediaElement): AudioGraphController {
  const existing = graphByElement.get(element)
  if (existing instanceof Error) throw existing
  if (existing) return existing
  try {
    const graph = new WebAudioGraph(element)
    graphByElement.set(element, graph)
    return graph
  } catch (cause) {
    const error = cause instanceof Error ? cause : new Error(String(cause))
    graphByElement.set(element, error)
    throw error
  }
}
