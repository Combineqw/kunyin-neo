<!--
  独立迷你播放器窗口视图：渲染主窗口推送的播放快照，并通过 IPC 回投控制命令。
  不持有音频元素、播放队列或曲库缓存，保持与主窗口播放状态的单向同步。
-->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { MiniPlayerState } from '@common'
import AppIcon from '../components/AppIcon.vue'
import { coverUrl } from '../utils/cover'

const state = ref<MiniPlayerState>({
  hasTrack: false,
  item: null,
  liked: false,
  title: '',
  artist: '',
  cover: '',
  currentTime: 0,
  duration: 0,
  playing: false
})
const expanded = ref(true)
let stateUnsub: (() => void) | null = null

const cover = computed(() => coverUrl(state.value.cover))
const progress = computed(() => {
  if (state.value.duration <= 0) return 0
  return Math.min(100, Math.max(0, (state.value.currentTime / state.value.duration) * 100))
})

function formatTime(value: number): string {
  const seconds = Math.max(0, Math.floor(value / 1000))
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
}

function sendCommand(command: 'playpause' | 'prev' | 'next'): void {
  window.api.miniPlayer.command(command)
}

function setExpanded(value: boolean): void {
  expanded.value = value
  window.api.miniPlayer.setExpanded(value)
}

async function toggleFavorite(): Promise<void> {
  if (!state.value.item) return
  const liked = await window.api.library.toggleFavorite(state.value.item)
  state.value = { ...state.value, liked }
}

onMounted(() => {
  stateUnsub = window.api.miniPlayer.onState((next) => {
    state.value = next
  })
  // 窗口初始按展开尺寸创建；这里显式同步 renderer 状态，避免首次 hover 状态不一致。
  window.api.miniPlayer.setExpanded(true)
})

onUnmounted(() => {
  stateUnsub?.()
  stateUnsub = null
})
</script>

<template>
  <main
    class="mini-player"
    :class="{ expanded, empty: !state.hasTrack }"
    @mouseenter="setExpanded(true)"
    @mouseleave="setExpanded(false)"
  >
    <div class="drag-region" aria-hidden="true"></div>
    <div class="cover-wrap">
      <img v-if="cover" class="cover" :src="cover" alt="" draggable="false" />
      <div v-else class="cover fallback"><AppIcon name="headphone" :size="26" /></div>
      <span v-if="state.playing" class="playing-mark" aria-hidden="true"></span>
    </div>

    <section class="details">
      <div class="track-title" :title="state.title">{{ state.title || '暂无播放' }}</div>
      <div class="track-artist" :title="state.artist">
        {{ state.artist || '选择一首歌曲开始播放' }}
      </div>
      <div class="progress-row">
        <div class="progress-track" aria-hidden="true">
          <span
            class="progress-value"
            :style="{ transform: `scaleX(${progress / 100})` }"
          ></span>
        </div>
        <span class="time"
          >{{ formatTime(state.currentTime) }} / {{ formatTime(state.duration) }}</span
        >
      </div>
    </section>

    <nav class="controls" aria-label="播放控制">
      <button
        class="control favorite pressable"
        :class="{ liked: state.liked }"
        :title="state.liked ? '取消收藏' : '收藏'"
        :aria-label="state.liked ? '取消收藏' : '收藏'"
        :disabled="!state.item"
        @click.stop="toggleFavorite"
      >
        <AppIcon :name="state.liked ? 'heart-filled' : 'heart'" :size="17" />
      </button>
      <button class="control pressable" title="上一首" aria-label="上一首" @click.stop="sendCommand('prev')">
        <AppIcon name="skip-back" :size="18" />
      </button>
      <button
        class="control primary pressable"
        :title="state.playing ? '暂停' : '播放'"
        :aria-label="state.playing ? '暂停' : '播放'"
        @click.stop="sendCommand('playpause')"
      >
        <AppIcon :name="state.playing ? 'pause' : 'play'" :size="20" />
      </button>
      <button class="control pressable" title="下一首" aria-label="下一首" @click.stop="sendCommand('next')">
        <AppIcon name="skip-forward" :size="18" />
      </button>
    </nav>
  </main>
</template>

<style scoped>
:global(*) {
  box-sizing: border-box;
}

:global(html),
:global(body) {
  overflow: hidden;
  background: transparent;
}

.mini-player {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  height: 100%;
  padding: 10px;
  overflow: hidden;
  color: #f8f7f1;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 18px;
  background: rgba(31, 33, 31, 0.93);
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.32);
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}

.mini-player:hover {
  border-color: rgba(232, 192, 131, 0.58);
  background: rgba(35, 37, 34, 0.97);
}

.drag-region {
  position: absolute;
  inset: 0;
  z-index: 0;
  -webkit-app-region: drag;
}

.cover-wrap,
.details,
.controls {
  position: relative;
  z-index: 1;
}

.cover-wrap {
  width: 70px;
  height: 70px;
  flex: 0 0 70px;
  overflow: hidden;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.12);
}

.cover,
.fallback {
  display: block;
  width: 100%;
  height: 100%;
}

.cover {
  object-fit: cover;
}

.fallback {
  display: grid;
  place-items: center;
  color: rgba(248, 247, 241, 0.7);
}

.playing-mark {
  position: absolute;
  right: 6px;
  bottom: 6px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #e8c083;
  box-shadow: 0 0 0 3px rgba(31, 33, 31, 0.62);
}

.details {
  min-width: 0;
  flex: 1;
}

.track-title,
.track-artist {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.track-title {
  color: #fffdf6;
  font-size: 14px;
  font-weight: 650;
  line-height: 20px;
}

.track-artist {
  margin-top: 1px;
  color: rgba(248, 247, 241, 0.64);
  font-size: 12px;
  line-height: 18px;
}

.progress-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}

.progress-track {
  width: 100%;
  height: 3px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.16);
}

.progress-value {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: #e8c083;
  transform-origin: left center;
  transition: transform var(--anim-dur-fast) linear;
}

.time {
  flex: none;
  color: rgba(248, 247, 241, 0.5);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}

.controls {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: none;
}

.control {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  padding: 0;
  color: rgba(248, 247, 241, 0.82);
  border: 0;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
  -webkit-app-region: no-drag;
}

.control:hover {
  color: #fffdf6;
  background: rgba(255, 255, 255, 0.12);
}

.control.favorite.liked {
  color: #e85c5c;
}

.control:disabled {
  opacity: 0.35;
  cursor: default;
}

.control.primary {
  color: #242621;
  background: #e8c083;
}

.control.primary:hover {
  background: #f2d19d;
}

.mini-player:not(.expanded) {
  justify-content: center;
  padding: 1px;
  border-radius: 16px;
  background: rgba(31, 33, 31, 0.84);
}

.mini-player:not(.expanded) .cover-wrap {
  width: 70px;
  height: 70px;
}

.mini-player:not(.expanded) .details,
.mini-player:not(.expanded) .controls {
  display: none;
}

.mini-player.empty:not(.expanded) .cover-wrap {
  opacity: 0.72;
}
</style>
