/**
 * 主窗口到迷你播放器窗口的状态桥接。
 * 迷你窗口只消费此处推送的快照并回传控制命令，播放队列和音频元素始终留在主窗口。
 */
import { onUnmounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { usePlayerStore } from '../stores/player'
import { useSettingsStore } from '../stores/settings'
import { useLibraryStore } from '../stores/library'
import type { MiniPlayerState } from '@common'
import { subscribeRuntimeSync } from './runtimeSyncScheduler'

/**
 * 主窗口到迷你播放器的轻量状态桥。
 * 迷你窗口不创建音频元素，也不持有播放队列，只显示状态并把控制命令回投主窗口。
 */
/**
 * 启动迷你播放器状态同步，并在所属组件卸载时停止进度定时器。
 *
 * @returns 不返回状态；副作用是向独立迷你窗口推送 IPC 快照。
 */
export function useMiniPlayerBridge(): void {
  const player = usePlayerStore()
  const settings = useSettingsStore()
  const library = useLibraryStore()
  const { current, playing, currentTime, duration } = storeToRefs(player)
  let unsubscribeSync: (() => void) | null = null
  let cachedTrack: unknown = undefined
  let cachedItem: MiniPlayerState['item'] = null

  function enabled(): boolean {
    return settings.settings.player.miniPlayerEnabled
  }

  function push(): void {
    if (!enabled()) return
    const track = current.value
    // The track object is stable while it plays. Clone it only when the
    // current item changes; cloning it on every progress tick creates a large
    // amount of short-lived JSON data and keeps the renderer's GC busy.
    if (track !== cachedTrack) {
      cachedTrack = track
      cachedItem = track ? JSON.parse(JSON.stringify(track)) : null
    }
    const state: MiniPlayerState = {
      hasTrack: !!track,
      item: cachedItem,
      liked: !!track && library.isFavorite(track),
      title: track?.title ?? '',
      artist: track?.artist ?? '',
      cover: track?.cover ?? '',
      currentTime: player.getAccurateCurrentTime(),
      duration: duration.value,
      playing: playing.value
    }
    window.api.miniPlayer.push(state)
  }

  function syncTimerState(): void {
    unsubscribeSync?.()
    unsubscribeSync = null
    if (enabled() && playing.value) unsubscribeSync = subscribeRuntimeSync(push)
  }

  watch(current, push)
  watch(() => library.favoriteKeys, push, { deep: true })
  watch(duration, push)
  watch(playing, () => {
    push()
    syncTimerState()
  })
  watch(currentTime, () => {
    if (!playing.value) push()
  })
  watch(
    () => settings.settings.player.miniPlayerEnabled,
    (on) => {
      if (on) push()
      syncTimerState()
    }
  )
  syncTimerState()

  onUnmounted(() => {
    unsubscribeSync?.()
  })
}
