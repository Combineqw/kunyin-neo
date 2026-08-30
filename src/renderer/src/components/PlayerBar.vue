<!--
  主布局底部播放条：消费播放器/曲库 store 的展示状态，触发既有播放、跳转和收藏操作。
  不创建音频元素，真实播放时序全部由 player store 管理。
-->
<script setup lang="ts">
import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import AppIcon from './AppIcon.vue'
import { usePlayerStore } from '../stores/player'
import { useLibraryStore } from '../stores/library'
import { coverUrl } from '../utils/cover'

const router = useRouter()
const player = usePlayerStore()
const library = useLibraryStore()
const { current, playing, currentTime, duration, volume, muted, playMode, error } =
  storeToRefs(player)

const liked = computed(() => (current.value ? library.isFavorite(current.value) : false))
function toggleLike(): void {
  if (current.value) void library.toggleFavorite(current.value)
}

const progress = computed(() =>
  duration.value > 0 ? Math.min(100, (currentTime.value / duration.value) * 100) : 0
)

function fmt(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return '--:--'
  const s = Math.floor(ms / 1000)
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
}
const timeText = computed(() => `${fmt(currentTime.value)} / ${fmt(duration.value)}`)

function openPlayer(): void {
  if (current.value) void router.push({ name: 'player' })
}

function onProgressInput(event: Event): void {
  if (!current.value || duration.value <= 0) return
  player.seek(Number((event.target as HTMLInputElement).value))
}

// 音量弹层
const volOpen = ref(false)
const volIcon = computed(() => (muted.value || volume.value === 0 ? 'volume-mute' : 'volume'))
let volCloseTimer: ReturnType<typeof setTimeout> | null = null
function openVol(): void {
  if (volCloseTimer) {
    clearTimeout(volCloseTimer)
    volCloseTimer = null
  }
  volOpen.value = true
}
function closeVol(): void {
  // 短延迟关闭：按钮→弹层斜向移动时短暂离界不闪关
  if (volCloseTimer) clearTimeout(volCloseTimer)
  volCloseTimer = setTimeout(() => (volOpen.value = false), 200)
}
function onVolInput(e: Event): void {
  player.setVolume(Number((e.target as HTMLInputElement).value))
}

// 播放模式
const PLAY_MODE_META: Record<string, { icon: string; label: string }> = {
  listLoop: { icon: 'repeat', label: '列表循环' },
  singleLoop: { icon: 'repeat-one', label: '单曲循环' },
  random: { icon: 'shuffle', label: '随机播放' },
  heartbeat: { icon: 'heartbeat', label: '心动模式' }
}
const modeMeta = computed(() => PLAY_MODE_META[playMode.value] ?? PLAY_MODE_META.listLoop)

async function cyclePlayMode(): Promise<void> {
  const modes = ['listLoop', 'singleLoop', 'random', 'heartbeat']
  const next = modes[(modes.indexOf(playMode.value) + 1) % modes.length]
  if (next !== 'heartbeat') {
    player.cyclePlayMode()
    return
  }
  const result = await player.startHeartbeatMode()
  if (result === 'insufficient-data') {
    const continueAnyway = window.confirm(
      '喜欢的歌曲少于 20 首，推荐仍在积累中，结果会偏随机。仍要开启心动模式吗？'
    )
    if (continueAnyway) {
      const retry = await player.startHeartbeatMode(true)
      if (retry === 'no-match') window.alert('当前歌曲缺少可用于心动模式的本地推荐数据。')
    }
    return
  }
  if (result === 'disabled') window.alert('心动模式已在设置中关闭。')
  else if (result === 'no-match') window.alert('当前歌曲缺少可用于心动模式的本地推荐数据。')
}
</script>

<template>
  <footer class="player">
    <div class="progress" :class="{ disabled: !current || duration <= 0 }">
      <div class="progress-fill" :style="{ transform: `scaleX(${progress / 100})` }" />
      <input
        class="progress-input"
        type="range"
        min="0"
        :max="duration"
        step="1000"
        :value="currentTime"
        :disabled="!current || duration <= 0"
        aria-label="播放进度"
        @input="onProgressInput"
      />
    </div>

    <!-- LX FullWidthProgress：封面 | 信息 | 时间 | 小控制 | 播放键 -->
    <!-- 无曲目时不可点，也就不给按压反馈 -->
    <div class="cover" :class="{ clickable: current, pressable: current }" @click="openPlayer">
      <img v-if="current?.cover" :src="coverUrl(current.cover)" alt="" />
      <div v-else class="cover-empty"><AppIcon name="library" :size="18" /></div>
    </div>

    <div class="info">
      <div class="title ellipsis">{{ current?.title || '未在播放' }}</div>
      <!-- 播放失败时占用艺术家行：失败原因必须可见，否则用户只能靠 devtools 猜 -->
      <div v-if="error" class="err ellipsis" :title="error">{{ error }}</div>
      <div v-else class="artist ellipsis">{{ current?.artist || '选一首歌开始' }}</div>
    </div>

    <div class="time">{{ timeText }}</div>

    <div class="controls">
      <button
        class="act pressable"
        :class="{ liked }"
        title="收藏"
        :disabled="!current"
        @click="toggleLike"
      >
        <AppIcon :name="liked ? 'heart-filled' : 'heart'" :size="17" />
      </button>
      <button class="act pressable" :title="modeMeta.label" @click="cyclePlayMode">
        <AppIcon :name="modeMeta.icon" :size="17" />
      </button>
      <!-- .vol-wrap 是 .vol-pop 的定位祖先，不能缩放；只把按钮做成 pressable -->
      <div class="vol-wrap" @mouseenter="openVol" @mouseleave="closeVol">
        <button class="act pressable" title="音量" @click="player.toggleMute()">
          <AppIcon :name="volIcon" :size="17" />
        </button>
        <div v-show="volOpen" class="vol-pop">
          <input
            class="vol-slider"
            type="range"
            min="0"
            max="1"
            step="0.01"
            :value="muted ? 0 : volume"
            @input="onVolInput"
          />
        </div>
      </div>
    </div>

    <div class="play-btns">
      <button class="ctrl pressable" title="上一首" @click="player.prev()">
        <AppIcon name="skip-back" :size="19" />
      </button>
      <button class="play pressable" :title="playing ? '暂停' : '播放'" @click="player.toggle()">
        <AppIcon :name="playing ? 'pause' : 'play'" :size="25" />
      </button>
      <button class="ctrl pressable" title="下一首" @click="player.next()">
        <AppIcon name="skip-forward" :size="19" />
      </button>
    </div>
  </footer>
</template>

<style scoped>
.player {
  position: relative;
  flex: none;
  height: var(--height-player);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 6px 6px;
}
.progress {
  position: absolute;
  z-index: 2;
  top: 0;
  left: 0;
  width: 100%;
  height: 4px;
  background-color: var(--color-primary-background);
}
.progress-fill {
  position: absolute;
  inset: 0;
  transform-origin: left center;
  background-color: var(--color-primary);
  transition: transform var(--anim-dur-fast) linear;
  pointer-events: none;
}
.progress-input {
  position: absolute;
  inset: -5px 0;
  width: 100%;
  height: 14px;
  margin: 0;
  opacity: 0;
  cursor: pointer;
}
.progress-input:disabled {
  cursor: default;
}
.progress:hover {
  height: 6px;
}
.progress:hover .progress-input {
  inset: -4px 0;
  height: 14px;
}
.progress.disabled {
  opacity: 0.55;
}

/* 封面：高=栏内容高，正方形 */
.cover {
  flex: none;
  height: 100%;
  aspect-ratio: 1 / 1;
  border-radius: var(--radius-border);
  overflow: hidden;
  box-shadow: 0 0 2px rgba(0, 0, 0, 0.3);
}
.cover.clickable {
  cursor: pointer;
}
.cover.clickable:hover {
  box-shadow: 0 0 4px rgba(0, 0, 0, 0.4);
}
.cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.cover-empty {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-font-label);
  background-color: var(--color-primary-light-900-alpha-200);
}

.info {
  flex: 1;
  min-width: 0;
}
.title {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-font);
}
.artist {
  font-size: 12px;
  color: var(--color-font-label);
}
.err {
  font-size: 12px;
  color: #d9534f;
}

.time {
  flex: none;
  min-width: 94px;
  padding: 0 4px;
  font-size: 12px;
  color: var(--color-550);
  font-variant-numeric: tabular-nums;
  text-align: center;
}

/* 小控制按钮：24px 宽，opacity .6 → hover 1（LX ControlBtns） */
.controls {
  flex: none;
  display: flex;
  align-items: center;
  gap: 10px;
}
.act {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  color: var(--color-button-font);
  opacity: 0.6;
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.act:hover:not(:disabled) {
  opacity: 1;
}
.act.liked {
  color: var(--color-primary-font);
  opacity: 1;
}
.act:disabled {
  opacity: 0.3;
  cursor: default;
}

.vol-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.vol-pop {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  padding: 12px 8px;
  border-radius: var(--radius-border);
  background-color: var(--color-content-background);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
}
/* 隐形桥接区：填平按钮与弹层之间的空隙，移向滑杆途中不触发 mouseleave */
.vol-pop::after {
  content: '';
  position: absolute;
  top: 100%;
  left: -12px;
  right: -12px;
  height: 10px;
}
.vol-slider {
  writing-mode: vertical-lr;
  direction: rtl;
  width: 4px;
  height: 90px;
  accent-color: var(--color-primary);
  cursor: pointer;
}

/* 播放键组：最右侧，gap 18px，右 padding 15px */
.play-btns {
  flex: none;
  display: flex;
  align-items: center;
  gap: 18px;
  padding-right: 15px;
}
.ctrl {
  display: flex;
  color: var(--color-button-font);
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.ctrl:hover {
  color: var(--color-primary-font);
}
.play {
  display: flex;
  color: var(--color-primary-font);
}
.play:hover {
  color: var(--color-primary-font-hover);
}
</style>
