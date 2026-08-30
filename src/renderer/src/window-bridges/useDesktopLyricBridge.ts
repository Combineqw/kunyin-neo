/**
 * �����ڵ������ʴ��ڵ�״̬�Žӡ�
 * ��ģ��ֻ���� Pinia ����/����״̬��ͨ�� IPC ���Ϳ��գ���������ƵԪ�أ�Ҳ��������ʡ�
 */
import { onUnmounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { usePlayerStore } from '../stores/player'
import { useSettingsStore } from '../stores/settings'
import type { DesktopLyricState, Lyric } from '@common'

/**
 * 桌面歌词桥：主窗口侧全程运行，把当前歌词/进度/播放态推送给桌面歌词悬浮窗（经主进程转发）�?
 * 独立�? PlayerView 是否打开——切歌时拉一次歌词并缓存，进度变化时只推时间�?
 *
 * 仅当 settings.lyrics.desktopEnabled 时才拉歌�?/推送，省开销�?
 */
/**
 * 启动桌面歌词状态同步，并在所属组件卸载时自动释放定时器�?
 *
 * @returns 不返回状态；副作用是向独立歌词窗口推�? IPC 快照�?
 */
export function useDesktopLyricBridge(): void {
  const player = usePlayerStore()
  const settings = useSettingsStore()
  const { current, playing, currentTime } = storeToRefs(player)

  let cached: Lyric | null = null
  let loadToken = 0

  function enabled(): boolean {
    return settings.settings.lyrics.desktopEnabled
  }

  function push(): void {
    if (!enabled()) return
    const c = current.value
    const hasLyric = !!(cached && (cached.char || cached.lrc))
    const state: DesktopLyricState = {
      hasLyric,
      lyric: cached ? cached.char || cached.lrc : '',
      translate: cached?.trans ?? '',
      roman: (cached?.chroma || cached?.roma) ?? '',
      currentTime: player.getAccurateCurrentTime(),
      playing: playing.value,
      spectrum: settings.settings.lyrics.desktopAudioVisualization ? player.getSpectrumData() : [],
      title: c ? `${c.title} - ${c.artist}` : '',
      musicName: c?.title,
      musicSinger: c?.artist ? [c.artist] : []
    }
    window.api.desktopLyric.push(state)
  }

  /**
   * 为当前歌曲获取歌词并写入桥接缓存�?
   * 请求序号用于丢弃切歌后才返回的旧歌词，避免旧行文本覆盖新歌曲�?
   *
   * @returns 歌词加载和状态推送完成时兑现�?
   */
  async function loadLyric(): Promise<void> {
    const token = ++loadToken
    cached = null
    if (!current.value || !enabled()) {
      push()
      return
    }
    try {
      const plain = JSON.parse(JSON.stringify(current.value))
      const ly = await window.api.player.lyric(plain)
      if (token !== loadToken) return
      cached = ly
    } catch {
      cached = null
    }
    push()
  }

  let syncTimer: ReturnType<typeof setInterval> | null = null
  function syncTimerState(): void {
    if (syncTimer) clearInterval(syncTimer)
    syncTimer = null
    if (settings.settings.lyrics.desktopEnabled && playing.value) {
      // 播放时每 80ms 读取一次媒体元素真实时间；暂停态由状�?/seek 事件即时推送�?
      syncTimer = setInterval(push, 80)
    }
  }

  watch(current, () => void loadLyric())
  watch(playing, () => {
    push()
    syncTimerState()
  })
  // 暂停�? seek 后立即推送；播放态由 80ms 定时器读取更准确�? audio.currentTime�?
  watch(currentTime, () => {
    if (!playing.value) push()
  })
  // 开关打开时立即拉一次；频谱开关变化时下一�? 80ms 推送自然应用�?

  watch(
    () => settings.settings.lyrics.desktopEnabled,
    (on) => {
      if (on) void loadLyric()
      syncTimerState()
    }
  )
  syncTimerState()

  onUnmounted(() => {
    if (syncTimer) clearInterval(syncTimer)
  })
}
