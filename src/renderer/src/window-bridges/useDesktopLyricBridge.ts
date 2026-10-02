/** Main renderer to desktop lyrics/combined overlay state bridge. */
import { onUnmounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { usePlayerStore } from '../stores/player'
import { useSettingsStore } from '../stores/settings'
import { useLibraryStore } from '../stores/library'
import type { DesktopLyricState, Lyric } from '@common'
import { subscribeRuntimeSync } from './runtimeSyncScheduler'

/**
 * Pushes playback state to the desktop overlay while keeping audio and queue
 * ownership in the main renderer. Combined mode also consumes this payload.
 */
export function useDesktopLyricBridge(): void {
  const player = usePlayerStore()
  const settings = useSettingsStore()
  const library = useLibraryStore()
  const { current, playing, currentTime, duration } = storeToRefs(player)

  let cached: Lyric | null = null
  let cachedItem: DesktopLyricState['item'] = null
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
      musicSinger: c?.artist ? [c.artist] : [],
      cover: c?.cover ?? '',
      duration: duration.value,
      liked: !!c && library.isFavorite(c),
      item: cachedItem
    }
    window.api.desktopLyric.push(state)
  }

  /** Load lyrics for the current track and drop stale responses after a seek. */
  async function loadLyric(): Promise<void> {
    const token = ++loadToken
    cached = null
    cachedItem = current.value ? JSON.parse(JSON.stringify(current.value)) : null
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

  let unsubscribeSync: (() => void) | null = null
  function syncTimerState(): void {
    unsubscribeSync?.()
    unsubscribeSync = null
    if (settings.settings.lyrics.desktopEnabled && playing.value) {
      // Playback uses a shared 100ms tick; paused and seek states push immediately.
      unsubscribeSync = subscribeRuntimeSync(push)
    }
  }

  watch(current, () => void loadLyric())
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
    () => settings.settings.lyrics.desktopEnabled,
    (on) => {
      if (on) void loadLyric()
      syncTimerState()
    }
  )
  syncTimerState()

  onUnmounted(() => {
    unsubscribeSync?.()
  })
}
