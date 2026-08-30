import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { IrsProfile, MusicItem, PlayMode, QualityId, RecommendationSong } from '@common'
import {
  blockedQualityIds,
  createHeartbeatRecommendations,
  getMusicItemKey
} from '@common'
import { useSettingsStore } from './settings'
import { describeMediaError } from '../playback/mediaError'
import { QUALITY_LADDER } from '../playback/quality'
import { base64ToArrayBuffer } from '../audio/irsBuffer'
import {
  getOrCreateAudioGraph,
  type AudioGraphController,
  type AudioGraphState
} from '../audio/audioGraph'

/**
 * 播放器 Pinia 状态机与唯一 HTMLAudioElement 的编排层。
 * 播放地址经主进程自定义协议 kunyin:// 供 <audio>，规避 CSP 与 CDN 鉴权；
 * Web Audio 节点图、IRS 字节转换、音质阶梯及错误文案分别位于 audio/playback 模块。
 * currentTime/duration 单位为毫秒（与歌词引擎、UI 一致）。
 */
const SPECTRUM_BAR_COUNT = 20

/**
 * 播放队列的来源（哪个列表/试听/单曲）。各列表页据此判断「当前正在播放的队列属于哪里」，
 * 用于展示正在播放的列表标记、提示用户浏览的列表与播放队列不一致。
 * - local   本地歌单（含系统列表：我的收藏/试听列表）
 * - platform 在线歌单/专辑/歌手（id 统一为 `source:id`）
 * - trial   试听列表（搜索点单曲）
 * - single  单曲临时队列（重复歌曲弹窗等）
 */
export interface QueueSource {
  kind: 'local' | 'platform' | 'trial' | 'single' | 'heartbeat'
  id?: string
  name?: string
}

export const usePlayerStore = defineStore('player', () => {
  const current = ref<MusicItem | null>(null)
  const queue = ref<MusicItem[]>([])
  const index = ref(-1)
  /** 当前播放队列的来源列表（null 表示队列无来源，如恢复旧存档） */
  const queueSource = ref<QueueSource | null>(null)
  const playing = ref(false)
  const playMode = ref<PlayMode>('listLoop')
  const heartbeatSeed = ref<MusicItem | null>(null)
  const currentTime = ref(0)
  const duration = ref(0)
  const loading = ref(false)
  /** 播放地址解析失败原因（如卡密缺失） */
  const error = ref('')
  /** 逻辑音量 0..1；真实输出音量还会受 muted 影响。 */
  const volume = ref(useSettingsStore().settings.player.volume)
  const muted = ref(false)

  const audio = new Audio()
  // kunyin:// 响应带 Access-Control-Allow-Origin；anonymous 允许接入 Web Audio 主音频图。
  audio.crossOrigin = 'anonymous'
  audio.volume = volume.value
  let volumePersistChain: Promise<void> = Promise.resolve()

  function clampVolume(value: unknown): number {
    return Math.max(0, Math.min(1, typeof value === 'number' && Number.isFinite(value) ? value : 1))
  }

  function applyAudioVolume(): void {
    audio.volume = muted.value ? 0 : volume.value
  }

  function persistVolume(value: number): void {
    // 滑块 input 事件频率很高，串行化 IPC 写入，保证快速拖动时最后一次值不会
    // 被较早返回的请求覆盖。
    volumePersistChain = volumePersistChain
      .then(() => useSettingsStore().update({ player: { volume: value } }))
      .catch((cause) => {
        console.warn('[player] 持久化音量失败', cause)
      })
  }

  function syncVolumeFromSettings(): void {
    volume.value = clampVolume(useSettingsStore().settings.player.volume)
    applyAudioVolume()
  }

  // settings.load() 是异步的，而 player store 会在 App.vue setup 阶段先创建。
  // 监听设置值，确保磁盘值回来后同时更新滑块状态和真实 Audio.volume。
  watch(
    () => useSettingsStore().settings.player.volume,
    () => syncVolumeFromSettings(),
    { immediate: true }
  )

  // EQ、淡入淡出和频谱共用同一音频图。图只在用户激活后创建，避免自动播放策略挂起。
  let audioGraph: AudioGraphController | null = null
  let spectrumBytes: Uint8Array<ArrayBuffer> | null = null
  let loadedIrsId = ''
  let loadedIrsProfile: IrsProfile | null = null
  let irsLoadToken = 0
  let transitionToken = 0
  let activeMediaUrl = ''
  let sessionItem: MusicItem | null = null
  let sessionStartedAt = 0
  let sessionMaxPositionMs = 0

  /**
   * 延迟创建唯一共享音频图。
   * 必须绑定播放器唯一 audio 元素，才能避开 MediaElementAudioSourceNode 的单次创建限制。
   * @returns 可用的音频图控制器；浏览器不支持时返回 null 并保留原生播放降级。
   */
  function ensureAudioGraph(): AudioGraphController | null {
    if (audioGraph) return audioGraph
    try {
      audioGraph = getOrCreateAudioGraph(audio)
      spectrumBytes = new Uint8Array(audioGraph.analyser.frequencyBinCount)
      return audioGraph
    } catch (e) {
      console.warn('[player] 无法初始化共享音频图', e)
      return null
    }
  }

  /**
   * 将当前设置中的 IRS 预设解码后交给共享音频图。
   * 请求序号确保用户快速切换预设时，旧异步解码结果不会覆盖新选择。
   * @param profile 待加载预设；null 表示清空 ConvolverNode 的 buffer。
   */
  async function applyIrsProfile(profile: IrsProfile | null): Promise<void> {
    const graph = audioGraph
    if (!graph) return
    const token = ++irsLoadToken
    if (!profile) {
      loadedIrsId = ''
      loadedIrsProfile = null
      graph.setIrsBuffer(null)
      return
    }
    const profileChanged =
      loadedIrsId !== profile.id || loadedIrsProfile?.dataBase64 !== profile.dataBase64
    if (!profileChanged) return
    loadedIrsId = ''
    loadedIrsProfile = null
    graph.setIrsBuffer(null)
    try {
      const buffer = await graph.decodeIrs(base64ToArrayBuffer(profile.dataBase64))
      if (token !== irsLoadToken) return
      loadedIrsId = profile.id
      loadedIrsProfile = profile
      graph.setIrsBuffer(buffer)
    } catch (error) {
      if (token !== irsLoadToken) return
      console.warn('[player] IRS 解码失败', error)
    }
  }

  function applyAudioSettings(): void {
    const settings = useSettingsStore().settings.player
    const graph = audioGraph
    if (!graph) return
    graph.setEqualizerEnabled(settings.equalizerEnabled)
    graph.setBandGains(settings.equalizerGains)
    graph.setEqualizerProfile(settings.equalizerPreampDb, settings.equalizerFilters)
    graph.setSrsSettings({
      enabled: settings.srsEnabled,
      intensity: settings.srsIntensity,
      bass: settings.srsBass,
      voice: settings.srsVoice,
      treble: settings.srsTreble,
      space: settings.srsSpace,
      limiter: settings.srsLimiter
    })
    graph.setIrsSettings({ enabled: settings.irsEnabled, wetPercent: settings.irsWetPercent })
    const profile = settings.irsProfiles.find((item) => item.id === settings.irsProfileId) ?? null
    if (!profile || loadedIrsId !== profile.id) void applyIrsProfile(profile)
  }

  function enableAudioSpectrum(): void {
    const graph = ensureAudioGraph()
    if (!graph) return
    applyAudioSettings()
    void graph.resume().catch((e: unknown) => {
      console.warn('[player] 无法恢复共享音频图', e)
    })
  }

  watch(
    () => {
      const settings = useSettingsStore().settings.player
      return [
        settings.equalizerEnabled,
        settings.equalizerPreset,
        settings.equalizerPreampDb,
        JSON.stringify(settings.equalizerFilters),
        settings.srsEnabled,
        settings.srsIntensity,
        settings.srsBass,
        settings.srsVoice,
        settings.srsTreble,
        settings.srsSpace,
        settings.srsLimiter,
        settings.irsEnabled,
        settings.irsWetPercent,
        settings.irsDryPercent,
        settings.irsProfileId,
        JSON.stringify(settings.irsProfiles),
        ...settings.equalizerGains
      ]
    },
    () => applyAudioSettings(),
    { deep: true }
  )

  function recordCurrentSession(ended: boolean): void {
    const item = sessionItem
    if (!item) return
    const durationMs = Math.max(0, duration.value || item.duration)
    const playedMs = ended ? durationMs : Math.max(sessionMaxPositionMs, currentTime.value)
    void window.api.playlog
      .record({
        item: JSON.parse(JSON.stringify(item)) as MusicItem,
        playedAt: sessionStartedAt || Date.now(),
        durationMs,
        playedMs,
        ended
      })
      .catch(() => {})
    sessionItem = null
    sessionStartedAt = 0
    sessionMaxPositionMs = 0
  }

  function beginPlaySession(item: MusicItem): void {
    if (sessionItem && getMusicItemKey(sessionItem) !== getMusicItemKey(item)) recordCurrentSession(false)
    if (!sessionItem) {
      sessionItem = item
      sessionStartedAt = Date.now()
      sessionMaxPositionMs = 0
    }
  }

  function getSpectrumData(): number[] {
    const bars = new Array<number>(SPECTRUM_BAR_COUNT).fill(0)
    if (!audioGraph || !spectrumBytes || audio.paused || audioGraph.getState().contextState !== 'running')
      return bars
    audioGraph.analyser.getByteFrequencyData(spectrumBytes)
    // 频率桶按幂次划分：低频保留更多分辨率，高频合并，视觉上更接近真实音乐频谱。
    const usableBins = Math.min(spectrumBytes.length, 96)
    for (let i = 0; i < SPECTRUM_BAR_COUNT; i++) {
      const start = Math.max(1, Math.floor((i / SPECTRUM_BAR_COUNT) ** 1.55 * (usableBins - 1)))
      const end = Math.max(
        start + 1,
        Math.floor(((i + 1) / SPECTRUM_BAR_COUNT) ** 1.55 * (usableBins - 1))
      )
      let sum = 0
      let peak = 0
      for (let bin = start; bin <= Math.min(end, usableBins - 1); bin++) {
        const value = spectrumBytes[bin]
        sum += value
        if (value > peak) peak = value
      }
      const count = Math.max(1, Math.min(end, usableBins - 1) - start + 1)
      const level = (sum / count / 255) * 0.72 + (peak / 255) * 0.28
      bars[i] = Math.round(Math.max(0, Math.min(1, level)) * 1000) / 1000
    }
    return bars
  }

  window.addEventListener(
    'pointerdown',
    () => {
      if (useSettingsStore().settings.lyrics.desktopAudioVisualization) enableAudioSpectrum()
    },
    { capture: true }
  )

  audio.addEventListener('timeupdate', () => {
    currentTime.value = Math.floor(audio.currentTime * 1000)
    sessionMaxPositionMs = Math.max(sessionMaxPositionMs, currentTime.value)
    if (audio.paused) return
    // 进度实时落盘：独立小 key（每次约几十字节，timeupdate ~4 次/s 可忽略），
    // 完整状态含整个队列，高频序列化太贵，仍每 5s 兜底一次
    persistPosition()
    if (Date.now() - lastPersistAt > 5000) persistState()
  })
  audio.addEventListener('durationchange', () => {
    duration.value = Number.isFinite(audio.duration) ? Math.floor(audio.duration * 1000) : 0
  })
  audio.addEventListener('volumechange', () => {
    const actual = clampVolume(audio.volume)
    const expected = muted.value ? 0 : volume.value
    if (Math.abs(actual - expected) < 0.0001) return
    // 处理 audio element 被外部代码/系统媒体层改变的情况，避免只更新 UI
    // 而不落盘。外部把静音态改回非零时同步解除逻辑静音。
    if (actual > 0 && muted.value) muted.value = false
    volume.value = actual
    persistVolume(actual)
  })
  function isActiveMediaEvent(): boolean {
    return !activeMediaUrl || !audio.currentSrc || audio.currentSrc === activeMediaUrl
  }

  audio.addEventListener('play', () => {
    if (!isActiveMediaEvent()) return
    playing.value = true
    enableAudioSpectrum()
  })
  audio.addEventListener('pause', () => {
    if (!isActiveMediaEvent()) return
    playing.value = false
    persistState()
  })
  audio.addEventListener('ended', () => {
    if (!isActiveMediaEvent()) return
    recordCurrentSession(true)
    next()
  })
  audio.addEventListener('error', () => {
    if (!isActiveMediaEvent()) return
    // 解码/网络错误不等待渐变，立即静音，避免错误帧或残响继续输出。
    transitionToken += 1
    audioGraph?.silenceImmediately()
    const item = current.value
    const q = quality.value
    if (!item) return
    // 直链过期/403：使 URL 缓存失效并重试一次当前曲目。
    // 用 loadForResume 从出错位置续播（保持原音质优先）——直链过期多发生在播放中途，
    // 走 loadAndPlay 会把 currentTime 清零从头重播，随后的持久化再把 0 写回存档，进度就丢了。
    if (q && !retriedAfterError) {
      retriedAfterError = true
      // 修复: 错误恢复属于后台任务；消费 IPC 拒绝以避免再次失败时形成未处理 Promise。
      void window.api.player
        .invalidateUrl(JSON.parse(JSON.stringify(item)) as MusicItem, q)
        .catch((cause) => console.warn('[player] 播放地址缓存失效通知失败', cause))
      // 修复: 重试取流可能因网络或 IPC 拒绝失败，记录后仍保留现有错误事件处理流程。
      void loadForResume(item, currentTime.value, playing.value || !audio.paused, q).catch((cause) => {
        console.warn('[player] 播放错误恢复失败', cause)
      })
      return
    }
    // 重试后仍失败：如实报出来，别只留一条 console 错误
    // MEDIA_ERR_NETWORK/SRC_NOT_SUPPORTED 最常见的成因是代理不可用或直链被拒
    playing.value = false
    loading.value = false
    error.value = describeMediaError(audio.error)
  })

  // ============ 播放状态持久化（记住歌曲/队列/进度/静音） ============
  const SAVE_KEY = 'kunyin:playback'
  /** 实时进度存档（只含歌曲身份+进度的小对象，可承受 timeupdate 频率的写入） */
  const POS_KEY = 'kunyin:playback:pos'
  /** 当前实际播放的音质（播放页音质菜单高亮；错误重试时失效缓存用） */
  const quality = ref('')
  let retriedAfterError = false
  /** 加载序号：并发的 loadAndPlay / loadForResume 只认最后一次（见 loadAndPlay 注释） */
  let loadToken = 0
  let lastPersistAt = 0
  let persistTimer: ReturnType<typeof setTimeout> | null = null

  interface SavedPlayback {
    item: MusicItem
    queue: MusicItem[]
    index: number
    queueSource: QueueSource | null
    positionMs: number
    muted: boolean
    /** 上次实际播放的音质：恢复时优先尝试（可能与全局首选不同，如首选档缺失时的回退档） */
    quality?: string
  }

  function persistState(): void {
    lastPersistAt = Date.now()
    if (!current.value) return
    persistPosition() // 同步小存档，保证它永远不比完整存档旧（如暂停态拖进度条）
    const state: SavedPlayback = {
      item: current.value,
      queue: queue.value,
      index: index.value,
      queueSource: queueSource.value,
      positionMs: currentTime.value,
      muted: muted.value,
      quality: quality.value
    }
    try {
      localStorage.setItem(SAVE_KEY, JSON.stringify(state))
    } catch {
      /* 队列过大写不下则放弃 */
    }
  }
  /** 只写歌曲身份 + 进度的小存档（timeupdate 每次都调，恢复时校验身份再采用） */
  function persistPosition(): void {
    const c = current.value
    if (!c) return
    try {
      localStorage.setItem(
        POS_KEY,
        JSON.stringify({ id: c.id, type: c.type, positionMs: currentTime.value })
      )
    } catch {
      /* ignore */
    }
  }
  /** 切歌/队列变化后防抖保存（进度由 timeupdate 单独按 5s 节流） */
  function schedulePersist(): void {
    if (persistTimer) clearTimeout(persistTimer)
    persistTimer = setTimeout(persistState, 800)
  }
  window.addEventListener('beforeunload', () => {
    try {
      // invoke 是异步的，窗口销毁时可能来不及完成；用同步 IPC 把最后音量快照
      // 在主进程退出前落盘。正常调整仍由 persistVolume 的异步队列负责。
      window.api.settings.setSync({ player: { volume: volume.value } })
    } catch (cause) {
      console.warn('[player] 退出前同步持久化音量失败', cause)
    }
    persistState()
    recordCurrentSession(false)
  })

  /**
   * 音质尝试顺序：优先设置项，其余从低到高兜底（高音质常需卡密）。
   * 开启「屏蔽 AI 音质」后，被屏蔽的档位（全景声等）整条链路都不参与取流。
   */
  function qualityOrder(item: MusicItem): string[] {
    const settings = useSettingsStore().settings
    const preferred = settings.player.preferredQuality
    const blocked = blockedQualityIds(settings)
    const usable = (q: string): boolean => !!item.qualities[q] && !blocked.includes(q)
    const order: string[] = []
    if (usable(preferred)) order.push(preferred)
    for (const q of QUALITY_LADDER) if (usable(q) && !order.includes(q)) order.push(q)
    for (const q of Object.keys(item.qualities))
      if (!blocked.includes(q) && !order.includes(q)) order.push(q)
    return order.length ? order : ['128k']
  }

  /**
   * 「当前曲已变」判断：按歌曲身份（id+type）比，绝不能比对象引用——
   * current.value 里存的是 Vue reactive 代理，与调用方手里的原始对象（如 restore 的
   * JSON.parse 结果）引用不等，引用比较会把正常加载误判成过期而中途放弃。
   */
  function isCurrent(item: MusicItem): boolean {
    const c = current.value
    return !!c && c.id === item.id && c.type === item.type
  }

  async function fadeOutForTransition(): Promise<number> {
    const token = ++transitionToken
    const settings = useSettingsStore().settings.player
    const graph = ensureAudioGraph()
    if (settings.fadeEnabled && graph && !audio.paused && graph.getState().outputGain > 0.001) {
      await graph.fadeOut(settings.fadeDurationMs).catch(() => {})
    }
    return token
  }

  async function playWithFadeIn(token = transitionToken): Promise<boolean> {
    const settings = useSettingsStore().settings.player
    const graph = ensureAudioGraph()
    if (!graph || token !== transitionToken) return false
    applyAudioSettings()
    if (settings.fadeEnabled) graph.prepareFadeIn()
    else graph.restoreOutput()
    try {
      await graph.resume()
      if (token !== transitionToken) return false
      await audio.play()
      if (token !== transitionToken) return false
      if (settings.fadeEnabled) await graph.fadeIn(settings.fadeDurationMs)
      return token === transitionToken
    } catch (cause) {
      graph.silenceImmediately()
      console.warn('[player] 共享音频图无法开始播放', cause)
      return false
    }
  }

  async function pauseWithFadeOut(): Promise<void> {
    const token = await fadeOutForTransition()
    if (token !== transitionToken) return
    audio.pause()
    audioGraph?.restoreOutput()
  }

  async function loadAndPlay(item: MusicItem): Promise<void> {
    // 每次加载自增：错误重试与用户切歌都会 fire-and-forget 地调本函数，
    // 只靠 isCurrent 挡不住两个实例并发（那只在 await 返回后判一次，
    // 两边都可能通过），结果各自注册一路 kunyin:// 流、各自抢着设 audio.src——
    // DevTools 里就会看到同一首歌冒出多个 token 和一串「已取消」。
    const token = ++loadToken
    const transition = await fadeOutForTransition()
    const stale = (): boolean =>
      token !== loadToken || transition !== transitionToken || !isCurrent(item)

    if (stale()) return
    loading.value = true
    error.value = ''
    currentTime.value = 0
    duration.value = item.duration
    // 过 IPC 需普通对象（Pinia 响应式 Proxy 无法被 structuredClone）
    const plain = JSON.parse(JSON.stringify(item)) as MusicItem
    try {
      let lastReason = ''
      for (const q of qualityOrder(item)) {
        const res = await window.api.player.stream(plain, q)
        if (stale()) return
        if (res.ok) {
          quality.value = q
          activeMediaUrl = res.url
          audio.src = res.url
          audio.volume = muted.value ? 0 : volume.value
          const started = await playWithFadeIn(transition)
          if (stale()) return
          if (!started) {
            error.value = '音频上下文未激活，请再次点击播放'
            playing.value = false
          }
          loading.value = false
          return
        }
        lastReason = res.reason ?? ''
      }
      if (stale()) return
      error.value = lastReason || '无法播放'
      audio.pause()
      playing.value = false
    } finally {
      // 仅当自己仍是最新一次加载时才清 loading，避免把后继加载的状态覆盖掉
      if (token === loadToken) loading.value = false
    }
  }

  /**
   * 恢复上次会话：加载音频但不自动播放（依设置 autoPlay 决定是否续播），并 seek 到记忆进度。
   * @param preferQuality 优先尝试的音质（恢复会话时传上次实际播放档），失败再走常规顺序
   */
  async function loadForResume(
    item: MusicItem,
    positionMs: number,
    autoplay: boolean,
    preferQuality = ''
  ): Promise<void> {
    // 同 loadAndPlay：启动恢复可能与用户点歌并发，用序号保证只有最后一次生效
    const token = ++loadToken
    const transition = await fadeOutForTransition()
    const stale = (): boolean =>
      token !== loadToken || transition !== transitionToken || !isCurrent(item)

    if (stale()) return
    loading.value = true
    const plain = JSON.parse(JSON.stringify(item)) as MusicItem
    const base = qualityOrder(item)
    // preferQuality 来自上次播放/手动切档，需与 qualityOrder 同样受 AI 音质屏蔽约束
    const order =
      preferQuality && base.includes(preferQuality)
        ? [preferQuality, ...base.filter((q) => q !== preferQuality)]
        : base
    try {
      for (const q of order) {
        const res = await window.api.player.stream(plain, q)
        if (stale()) return
        if (res.ok) {
          quality.value = q
          activeMediaUrl = res.url
          audio.src = res.url
          audio.volume = muted.value ? 0 : volume.value
          // 等元数据就绪才能 seek；5s 超时兜底
          await new Promise<void>((resolve) => {
            const onMeta = (): void => {
              // 修复: 元数据先到达时清除 5 秒兜底，避免超时闭包继续持有播放器状态。
              clearTimeout(timeout)
              audio.removeEventListener('loadedmetadata', onMeta)
              resolve()
            }
            audio.addEventListener('loadedmetadata', onMeta)
            const timeout = setTimeout(() => {
              audio.removeEventListener('loadedmetadata', onMeta)
              resolve()
            }, 5000)
          })
          if (stale()) return
          if (positionMs > 0) seek(positionMs)
          const started = autoplay ? await playWithFadeIn(transition) : false
          if (!autoplay) audioGraph?.restoreOutput()
          if (stale()) return
          if (autoplay && !started) {
            // 自动播放策略阻止 AudioContext 时保留歌曲和进度，等待用户再次点击播放。
            playing.value = false
          }
          loading.value = false
          return
        }
      }
      if (stale()) return
      error.value = '无法恢复上次播放'
      audio.pause()
      playing.value = false
    } finally {
      if (token === loadToken) loading.value = false
    }
  }

  let restored = false
  /** 启动恢复：读出上次播放的歌曲/队列/进度（App.vue onMounted 调用一次） */
  async function restore(): Promise<void> {
    if (restored) return
    restored = true
    let saved: SavedPlayback | null = null
    try {
      const raw = localStorage.getItem(SAVE_KEY)
      saved = raw ? (JSON.parse(raw) as SavedPlayback) : null
    } catch {
      saved = null
    }
    if (!saved?.item) return
    // 实时进度小存档比完整存档新（完整存档 5s 一写），身份匹配当前曲时优先采用
    let positionMs = Math.max(0, saved.positionMs ?? 0)
    try {
      const rawPos = localStorage.getItem(POS_KEY)
      const pos = rawPos
        ? (JSON.parse(rawPos) as { id: number; type: string; positionMs: number })
        : null
      if (pos && pos.id === saved.item.id && pos.type === saved.item.type && pos.positionMs > 0) {
        positionMs = pos.positionMs
      }
    } catch {
      /* 小存档损坏则用完整存档的进度 */
    }
    queue.value = Array.isArray(saved.queue) ? saved.queue : []
    index.value = typeof saved.index === 'number' ? saved.index : -1
    queueSource.value = saved.queueSource ?? null
    muted.value = !!saved.muted
    syncVolumeFromSettings()
    current.value = saved.item
    beginPlaySession(saved.item)
    sessionMaxPositionMs = positionMs
    duration.value = saved.item.duration
    // 音质/进度先按记忆值上屏（菜单高亮 + 进度条归位）；解析直链失败也不至于显示回 0:00，
    // 更避免后续持久化把 0 写回存档覆盖掉真实进度。加载成功后由 loadForResume 内 seek 对齐。
    if (saved.quality) quality.value = saved.quality
    currentTime.value = positionMs
    const autoplay = useSettingsStore().settings.player.autoPlay
    await loadForResume(saved.item, positionMs, autoplay, saved.quality ?? '')
  }

  /**
   * 播放一首歌。
   * @param list 设为播放队列（浏览页传歌单/专辑等结果列表）
   * @param opts.source 队列来源列表（用于「正在播放的列表」标记；不传则视为无来源）
   * @param opts.trackTrial 是否把该曲累积进「试听列表」（专辑/歌手点单曲时作历史累积）。
   *   从试听列表自身播放时应传 false，避免自我扰动。默认 false（不动试听列表）。
   *   搜索点单曲请用 playInTrial——试听列表即其播放上下文，不只是历史记录。
   */
  function playItem(
    item: MusicItem,
    list?: MusicItem[],
    opts?: { trackTrial?: boolean; source?: QueueSource }
  ): void {
    // 必须在用户点击的同步调用栈中创建/恢复 AudioContext，避免异步取流后被自动播放策略挂起。
    enableAudioSpectrum()
    beginPlaySession(item)
    current.value = item
    retriedAfterError = false // 用户主动切歌：允许错误重试
    if (list) {
      queue.value = list
      // 用歌曲身份而非引用定位：队列可能是主进程重新序列化的对象（如试听列表），引用比较会失配
      index.value = Math.max(
        0,
        list.findIndex((m) => getMusicItemKey(m) === getMusicItemKey(item))
      )
      queueSource.value = opts?.source ?? null
    }
    schedulePersist()
    if (opts?.trackTrial) {
      // 追加到试听列表末尾（atHead=false，去重）—— 累积不覆盖、不打乱既有顺序
      const plain = JSON.parse(JSON.stringify(item)) as MusicItem
      void window.api.library.addToTrial(plain, false).catch(() => {})
    }
    // 修复: 点歌异步取流失败时消费拒绝，避免事件处理器外产生未处理 Promise。
    void loadAndPlay(item).catch((cause) => {
      console.warn('[player] 加载播放任务失败', cause)
    })
  }

  /**
   * 试听播放（搜索点单曲）：把该曲累积进试听列表，再以**整个试听列表**为队列播放——
   * 试听列表即试听场景的播放上下文（对齐 LX）：上一首/下一首在试听历史内导航，
   * 播完自动接下一首，而不是单曲循环或跳进其他搜索结果。
   * 拉取失败时退化为只放该曲，保证可播。
   */
  async function playInTrial(item: MusicItem): Promise<void> {
    enableAudioSpectrum()
    // 过 IPC 需普通对象（Pinia 响应式 Proxy 无法被 structuredClone）
    const plain = JSON.parse(JSON.stringify(item)) as MusicItem
    // 先累积再拉列表：await 串行保证队列里一定包含本曲。
    // 主进程 INSERT OR IGNORE 去重——已存在的歌保持原位，不会被挪到末尾。
    await window.api.library.addToTrial(plain, false).catch(() => {})
    const list = await window.api.library.trialSongs().catch(() => [] as MusicItem[])
    const q = list.length ? list : [plain]
    beginPlaySession(item)
    current.value = item
    retriedAfterError = false // 用户主动切歌：允许错误重试
    queue.value = q
    queueSource.value = { kind: 'trial', name: '试听列表' }
    // trialSongs 是主进程新序列化出的对象，引用比较找不到，按歌曲身份定位
    const key = getMusicItemKey(item)
    index.value = Math.max(
      0,
      q.findIndex((m) => getMusicItemKey(m) === key)
    )
    schedulePersist()
    // 修复: 试听取流在后台执行；消费拒绝避免网络或 IPC 失败泄漏为未处理 Promise。
    void loadAndPlay(item).catch((cause) => {
      console.warn('[player] 试听加载播放任务失败', cause)
    })
  }

  function toggle(): void {
    if (!current.value) return
    enableAudioSpectrum()
    if (audio.paused) {
      const token = ++transitionToken
      void playWithFadeIn(token).catch(() => {})
    } else {
      void pauseWithFadeOut()
    }
  }

  /**
   * 「下一首播放」：把歌曲插入队列，紧跟当前曲之后，不打断当前播放。
   * 队列为空或未在播放时，直接开始播放该曲。已在队列中的先去重再插入。
   */
  function playNext(item: MusicItem): void {
    if (!current.value || !queue.value.length) {
      playItem(item, [item], { source: { kind: 'single' } })
      return
    }
    const key = (m: MusicItem): string => `${m.id}_${m.type}`
    const q = queue.value.slice()
    const dup = q.findIndex((m) => key(m) === key(item))
    if (dup >= 0 && dup <= index.value) {
      // 去重项在当前之前，移除会使 index 前移一位
      q.splice(dup, 1)
      index.value -= 1
    } else if (dup > index.value) {
      q.splice(dup, 1)
    }
    q.splice(index.value + 1, 0, item)
    queue.value = q
  }

  /**
   * 从播放队列移除一首歌（列表删除歌曲时同步调用）。
   * 删除的是当前播放曲时，立即接播队列「下一首」（队列已空则停止），不再播已删的歌；
   * 删除其他位置的曲只做数组剔除，index 保持指向不变，prev/next 也切不到已删的这首。
   */
  function removeFromQueue(item: MusicItem): void {
    const key = getMusicItemKey(item)
    const idx = queue.value.findIndex((m) => getMusicItemKey(m) === key)
    if (idx < 0) return
    const removingCurrent =
      idx === index.value && !!current.value && getMusicItemKey(current.value) === key
    queue.value.splice(idx, 1)
    if (idx <= index.value) index.value -= 1
    if (index.value < 0) index.value = queue.value.length ? 0 : -1
    schedulePersist()
    if (removingCurrent) {
      if (queue.value.length)
        step(1) // 删除即切走，忽略 singleLoop 的重播语义
      else void pauseWithFadeOut()
    }
  }

  function seek(ms: number): void {
    if (!current.value) return
    audio.currentTime = Math.max(0, ms) / 1000
    currentTime.value = Math.max(0, ms)
    // 暂停态拖进度条后 timeupdate 不跑、pause 也不会再触发，异常退出就会丢掉新位置
    schedulePersist()
  }

  async function rebuildHeartbeatQueue(seedItem: MusicItem, autoplay: boolean): Promise<boolean> {
    const songs = await window.api.recommendation.songs().catch(() => [] as RecommendationSong[])
    const seed = songs.find((song) => getMusicItemKey(song.item) === getMusicItemKey(seedItem))
    if (!seed) return false
    const result = createHeartbeatRecommendations(seed, songs, { limit: 30, familiarRatio: 0.7 })
    const candidates = result.map((entry) => entry.song.item)
    if (!candidates.length) return false
    heartbeatSeed.value = seedItem
    queueSource.value = { kind: 'heartbeat', name: '心动模式' }
    if (autoplay) {
      // 种子刚刚播完，只用于计算下一轮相似度；从首个新候选继续，绝不重播种子。
      queue.value = candidates
      index.value = 0
      playItem(candidates[0], candidates, { source: queueSource.value })
    } else {
      // 收藏变更时保留当前正在播的种子，只重排后续候选。
      queue.value = [seedItem, ...candidates]
      index.value = 0
      schedulePersist()
    }
    return true
  }

  async function startHeartbeatMode(
    allowSparseData = false
  ): Promise<'started' | 'insufficient-data' | 'disabled' | 'no-match'> {
    if (!useSettingsStore().settings.player.heartbeatEnabled) return 'disabled'
    const item = current.value
    if (!item) return 'no-match'
    const songs = await window.api.recommendation.songs().catch(() => [] as RecommendationSong[])
    const favoriteCount = songs.filter((song) => song.isFavorite).length
    if (favoriteCount < 20 && !allowSparseData) return 'insufficient-data'
    const seed = songs.find((song) => getMusicItemKey(song.item) === getMusicItemKey(item))
    if (!seed) return 'no-match'
    const result = createHeartbeatRecommendations(seed, songs, { limit: 30, familiarRatio: 0.7 })
    if (!result.length) return 'no-match'
    heartbeatSeed.value = item
    playMode.value = 'heartbeat'
    queue.value = [item, ...result.map((entry) => entry.song.item)]
    index.value = 0
    queueSource.value = { kind: 'heartbeat', name: '心动模式' }
    schedulePersist()
    // 修复: 设置持久化经 IPC 异步执行；心动模式已就绪时仅记录持久化失败，避免未处理拒绝。
    void useSettingsStore()
      .update({ player: { playMode: 'heartbeat' } })
      .catch((cause) => console.warn('[player] 持久化心动模式失败', cause))
    return 'started'
  }

  async function refreshHeartbeatAfterFavoriteChange(): Promise<void> {
    if (playMode.value !== 'heartbeat' || !current.value) return
    await rebuildHeartbeatQueue(current.value, false)
  }

  function step(delta: number): void {
    if (!queue.value.length) return
    const n = queue.value.length
    let i = index.value + delta
    if (playMode.value === 'random') i = Math.floor(Math.random() * n)
    if (i < 0) i = n - 1
    if (i >= n) i = 0
    index.value = i
    // 在队列内切歌不改来源；source 回传保持「正在播放的列表」标记稳定
    playItem(queue.value[i], queue.value, { source: queueSource.value ?? undefined })
  }
  function next(): void {
    if (playMode.value === 'heartbeat' && index.value >= queue.value.length - 1 && current.value) {
      void rebuildHeartbeatQueue(current.value, true)
      return
    }
    if (playMode.value === 'singleLoop' && current.value) {
      recordCurrentSession(true)
      beginPlaySession(current.value)
      seek(0)
      const token = ++transitionToken
      void playWithFadeIn(token).catch(() => {})
      return
    }
    step(1)
  }
  function prev(): void {
    step(-1)
  }

  function setEqualizerEnabled(enabled: boolean): void {
    ensureAudioGraph()?.setEqualizerEnabled(enabled)
  }

  function setEqualizerBand(index: number, gain: number): void {
    ensureAudioGraph()?.setBandGain(index, gain)
  }

  function setEqualizerBands(gains: readonly number[]): void {
    ensureAudioGraph()?.setBandGains(gains)
  }

  async function previewAudioEffect(): Promise<number[]> {
    const graph = ensureAudioGraph()
    if (!graph) throw new Error('当前环境无法初始化 Web Audio 音效链')
    applyAudioSettings()
    const settings = useSettingsStore().settings.player
    const profile = settings.irsProfiles.find((item) => item.id === settings.irsProfileId) ?? null
    await applyIrsProfile(profile)
    await graph.previewEffect(1200)
    return graph.getIrsResponseCurve()
  }

  function getIrsResponseCurve(): number[] {
    return audioGraph?.getIrsResponseCurve() ?? []
  }

  function getAudioGraphState(): AudioGraphState | null {
    return audioGraph?.getState() ?? null
  }

  /** 设音量（0..1），实时应用到 audio 并持久化到设置 */
  function setVolume(v: number): void {
    const clamped = clampVolume(v)
    volume.value = clamped
    if (clamped > 0) muted.value = false
    applyAudioVolume()
    persistVolume(clamped)
  }
  /** 应用内快捷键调整音量；增量仍通过 setVolume 进入同一持久化链。 */
  function adjustVolume(delta: number): void {
    setVolume(volume.value + delta)
  }
  function toggleMute(): void {
    muted.value = !muted.value
    applyAudioVolume()
    // 静音状态属于播放会话存档；在隐藏到托盘、不触发 beforeunload 时也及时保存。
    persistState()
  }

  const PLAY_MODES: PlayMode[] = ['listLoop', 'singleLoop', 'random', 'heartbeat']
  function cyclePlayMode(): void {
    const i = PLAY_MODES.indexOf(playMode.value)
    playMode.value = PLAY_MODES[(i + 1) % PLAY_MODES.length]
    // 修复: 模式切换已在内存生效；消费异步持久化拒绝，避免后台失败冒泡为未处理 Promise。
    void useSettingsStore()
      .update({ player: { playMode: playMode.value } })
      .catch((cause) => console.warn('[player] 持久化播放模式失败', cause))
  }

  /** 跨窗口同步读取媒体元素的真实进度，避免受 timeupdate 约 4Hz 的更新频率限制。 */
  function getAccurateCurrentTime(): number {
    if ((!audio.currentSrc && !audio.src) || audio.readyState === 0) return currentTime.value
    const mediaTime = audio.currentTime * 1000
    return Number.isFinite(mediaTime) ? Math.max(0, Math.floor(mediaTime)) : currentTime.value
  }

  /** 切换播放音质：写入首选音质设置，并按当前进度重新解析当前曲目（缺该档位时自动回退） */
  async function changeQuality(q: QualityId): Promise<void> {
    await useSettingsStore().update({ player: { preferredQuality: q } })
    const item = current.value
    if (!item) return
    await loadForResume(item, currentTime.value, playing.value)
  }

  if (import.meta.env.DEV) {
    Object.assign(window, {
      __kunyinAudioDebug: {
        getState: getAudioGraphState,
        getVolume: () => ({ logical: volume.value, actual: audio.volume, muted: muted.value }),
        enableEqualizer: setEqualizerEnabled,
        setBand: setEqualizerBand,
        setBands: setEqualizerBands
      }
    })
  }

  return {
    current,
    queue,
    index,
    queueSource,
    playing,
    playMode,
    heartbeatSeed,
    currentTime,
    duration,
    loading,
    error,
    volume,
    muted,
    quality,
    enableAudioSpectrum,
    getSpectrumData,
    setEqualizerEnabled,
    setEqualizerBand,
    setEqualizerBands,
    getAudioGraphState,
    getIrsResponseCurve,
    previewAudioEffect,
    getAccurateCurrentTime,
    playItem,
    playInTrial,
    startHeartbeatMode,
    refreshHeartbeatAfterFavoriteChange,
    playNext,
    removeFromQueue,
    toggle,
    seek,
    next,
    prev,
    setVolume,
    adjustVolume,
    toggleMute,
    cyclePlayMode,
    changeQuality,
    restore
  }
})
