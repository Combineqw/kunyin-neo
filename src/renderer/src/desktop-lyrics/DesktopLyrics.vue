<!--
  独立桌面歌词窗口视图：消费主窗口推送的歌词快照，管理逐字歌词展示与窗口级外观控件。
  音频播放、歌词获取和跨窗口状态同步均不在此组件内处理。
-->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import { useLyricPlayer } from '../composables/useLyricPlayer'
import AppIcon from '../components/AppIcon.vue'
import type { AppSettings, DesktopLyricState } from '@common'

// 保留主窗口同款逐字歌词引擎，只把 LX Music 的桌面歌词外观与窗口选项接进来。
const host = ref<HTMLElement>()
const lyric = useLyricPlayer()
const appSettings = shallowRef<AppSettings | null>(null)
const hasLyric = ref(false)
const title = ref('')
const playing = ref(false)
const spectrum = ref<number[]>([])

let lastLyricKey = ''
let lastTime = 0
let lastStateAt = 0

const lyricSettings = computed(() => appSettings.value?.lyrics)
const locked = computed(() => lyricSettings.value?.desktopLocked ?? false)
const pauseHidden = computed(
  () => !!lyricSettings.value?.desktopPauseHide && !playing.value && !!title.value
)

const rootClasses = computed(() => {
  const s = lyricSettings.value
  return {
    locked: locked.value,
    'pause-hidden': pauseHidden.value,
    'hover-hide': !!s?.desktopHoverHide,
    'direction-vertical': s?.desktopDirection === 'vertical',
    ellipsis: !!s?.desktopEllipsis,
    'gradient-bar': !!s?.desktopGradientBar,
    'text-shadow': !!s?.desktopShadowEnabled,
    'bold-syllable': !!s?.desktopBoldSyllable,
    'bold-line': !!s?.desktopBoldLine,
    'bold-extended': !!s?.desktopBoldExtended
  }
})

const rootStyle = computed<Record<string, string>>(() => {
  const s = lyricSettings.value
  const bgOpacity = Math.max(0, Math.min(0.8, s?.desktopBgOpacity ?? 0.28))
  const shadowOpacity = Math.max(0, Math.min(1, s?.desktopShadowOpacity ?? 0.72))
  const gradientWidth = Math.max(0, Math.min(100, s?.desktopGradientWidth ?? 72))
  const gradientOpacity = Math.max(0, Math.min(1, s?.desktopGradientOpacity ?? 0.52))
  return {
    background: locked.value ? 'transparent' : `rgba(9, 12, 11, ${bgOpacity})`,
    '--dl-text-opacity': String(Math.max(0.06, Math.min(1, (s?.desktopOpacity ?? 100) / 100))),
    '--dl-font-weight': String(Math.max(100, Math.min(900, s?.desktopFontWeight ?? 600))),
    '--dl-stroke-color': s?.desktopStrokeColor || '#151515',
    '--dl-stroke-width': `${Math.max(0, Math.min(4, s?.desktopStrokeWidth ?? 0))}px`,
    '--dl-shadow-x': `${Math.max(-12, Math.min(12, s?.desktopShadowX ?? 0))}px`,
    '--dl-shadow-y': `${Math.max(-12, Math.min(12, s?.desktopShadowY ?? 2))}px`,
    '--dl-shadow-blur': `${Math.max(0, Math.min(24, s?.desktopShadowBlur ?? 6))}px`,
    '--dl-shadow-opacity': String(shadowOpacity),
    '--dl-gradient-edge': `${(100 - gradientWidth) / 2}%`,
    '--dl-gradient-opacity': String(gradientOpacity)
  }
})

function applyAppearance(settings: AppSettings): void {
  appSettings.value = settings
  const s = settings.lyrics
  lyric.setColors(s.desktopColorNormal || '#F5F5F2', s.desktopColorActive || '#E8C083')
  lyric.setFontFamily(s.desktopFont || s.font)
  lyric.setPresentation({
    fontSize: s.desktopFontSize,
    gap: s.desktopLineGap,
    align: s.desktopAlign,
    scrollAlign: s.desktopScrollAlign,
    delayScroll: s.desktopDelayScroll,
    zoomActive: s.desktopZoomActive
  })
  lyric.setAnnotationVisible({
    translation: s.showTranslation,
    romanization: s.showRomanization
  })
  requestAnimationFrame(() => lyric.relayout())
}

async function apply(state: DesktopLyricState): Promise<void> {
  const receivedAt = performance.now()
  title.value = state.title
  playing.value = state.playing
  spectrum.value = state.spectrum ?? []
  const key = `${state.lyric}\0${state.translate}\0${state.roman}\0${state.musicName ?? ''}\0${state.musicSinger?.join('/') ?? ''}`
  if (key !== lastLyricKey) {
    lastLyricKey = key
    if (state.hasLyric && state.lyric) {
      hasLyric.value = true
      await nextTick()
      lyric.loadLyric(state.lyric, state.translate, state.roman, {
        name: state.musicName,
        singer: state.musicSinger
      })
      await nextTick()
      requestAnimationFrame(() => lyric.relayout())
    } else {
      lyric.clear()
      hasLyric.value = false
    }
  }
  if (hasLyric.value) {
    if (state.playing) {
      // 以「歌词引擎是否在播」为准而非宿主上次推送的 playing：切歌后引擎 updateLyric
      // 内部会回到暂停，必须重新 play；宿主 playing 在歌词异步拉取期间已被推成 true，
      // 若再依赖它会被中间态卡住导致首次播放永不启动（歌词停在第一行不推进）。
      const mediaDelta = state.currentTime - lastTime
      const clockDelta = lastStateAt ? receivedAt - lastStateAt : mediaDelta
      if (!lyric.playing.value || Math.abs(mediaDelta - clockDelta) > 100) {
        lyric.play(state.currentTime)
      }
    } else {
      lyric.seekMs(state.currentTime)
    }
  }
  lastTime = state.currentTime
  lastStateAt = receivedAt
}

let stateUnsub: (() => void) | null = null
let settingsUnsub: (() => void) | null = null

onMounted(async () => {
  host.value?.appendChild(lyric.element.value)
  stateUnsub = window.api.desktopLyric.onState(apply)
  applyAppearance(await window.api.settings.get())
  settingsUnsub = window.api.settings.onChange(applyAppearance)
})

onUnmounted(() => {
  stateUnsub?.()
  settingsUnsub?.()
})

function updateLyrics(patch: Partial<AppSettings['lyrics']>): void {
  void window.api.settings.set({ lyrics: patch })
}

function lock(): void {
  window.api.desktopLyric.setLock(true)
}

function close(): void {
  void window.api.desktopLyric.toggle(false)
}

function changeFontSize(step: number): void {
  const value = lyricSettings.value?.desktopFontSize ?? 28
  updateLyrics({ desktopFontSize: Math.max(10, Math.min(80, value + step)) })
}

function changeOpacity(step: number): void {
  const value = lyricSettings.value?.desktopOpacity ?? 100
  updateLyrics({ desktopOpacity: Math.max(6, Math.min(100, value + step)) })
}

function toggleZoom(): void {
  updateLyrics({ desktopZoomActive: !lyricSettings.value?.desktopZoomActive })
}

function toggleAlwaysOnTop(): void {
  updateLyrics({ desktopAlwaysOnTop: !lyricSettings.value?.desktopAlwaysOnTop })
}
</script>

<template>
  <div class="dl-root" :class="rootClasses" :style="rootStyle">
    <div v-if="!locked" class="dl-toolbar">
      <div class="dl-drag">
        <span class="dl-caption">{{ title || '坤音neo · 桌面歌词' }}</span>
      </div>
      <div class="dl-actions">
        <button class="dl-btn text-btn" title="增大字号" @click="changeFontSize(1)">A⁺</button>
        <button class="dl-btn text-btn" title="减小字号" @click="changeFontSize(-1)">A⁻</button>
        <button class="dl-btn text-btn" title="提高歌词不透明度" @click="changeOpacity(10)">
          ◐⁺
        </button>
        <button class="dl-btn text-btn" title="降低歌词不透明度" @click="changeOpacity(-10)">
          ◐⁻
        </button>
        <button
          class="dl-btn text-btn wide"
          :class="{ active: lyricSettings?.desktopZoomActive }"
          title="切换当前行放大"
          @click="toggleZoom"
        >
          当前行
        </button>
        <button
          class="dl-btn text-btn wide"
          :class="{ active: lyricSettings?.desktopAlwaysOnTop }"
          title="切换窗口置顶"
          @click="toggleAlwaysOnTop"
        >
          置顶
        </button>
        <button class="dl-btn" title="锁定并允许点击穿透" @click="lock">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <rect x="5" y="10" width="14" height="10" rx="2" />
            <path d="M8 10V7a4 4 0 0 1 8 0v3" />
          </svg>
        </button>
        <button class="dl-btn" title="关闭桌面歌词" @click="close">
          <AppIcon name="close" :size="15" />
        </button>
      </div>
    </div>

    <div ref="host" class="dl-lyric" :class="{ hidden: !hasLyric }" />
    <div v-if="!hasLyric" class="dl-placeholder">
      <span>{{ title || '坤音neo · 桌面歌词' }}</span>
      <small>等待播放</small>
    </div>

    <div
      v-if="lyricSettings?.desktopAudioVisualization"
      class="dl-visualizer"
      :class="{ paused: !playing }"
      aria-hidden="true"
    >
      <i
        v-for="i in 20"
        :key="i"
        :style="{ '--bar-level': String(Math.max(0, Math.min(1, spectrum[i - 1] ?? 0))) }"
      />
    </div>
  </div>
</template>

<style scoped>
.dl-root {
  position: relative;
  width: 100%;
  height: 100%;
  color: #fff;
  border-radius: 10px;
  overflow: hidden;
  backdrop-filter: blur(10px) saturate(1.08);
  opacity: 1;
  transition: opacity 0.28s ease;
}
.dl-root.locked {
  border-radius: 0;
  backdrop-filter: none;
}
.dl-root.pause-hidden {
  opacity: 0.04;
}
.dl-root.pause-hidden:not(.locked):not(.hover-hide):hover {
  opacity: 1;
}
.dl-root.hover-hide:hover {
  opacity: 0.06;
}

.dl-toolbar {
  position: absolute;
  inset: 0 0 auto;
  z-index: 10;
  height: 36px;
  display: flex;
  align-items: center;
  color: rgba(255, 255, 255, 0.9);
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.76), rgba(0, 0, 0, 0.52));
  opacity: 0;
  transform: translateY(-4px);
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}
.dl-root:hover .dl-toolbar {
  opacity: 1;
  transform: none;
}
.dl-drag {
  min-width: 36px;
  height: 100%;
  flex: 1;
  display: flex;
  align-items: center;
  padding-left: 12px;
  -webkit-app-region: drag;
}
.dl-caption {
  max-width: 280px;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  font-size: 11px;
  opacity: 0.62;
}
.dl-actions {
  flex: none;
  height: 100%;
  display: flex;
  align-items: center;
  padding-right: 6px;
  -webkit-app-region: no-drag;
}
.dl-btn {
  width: 30px;
  height: 28px;
  display: grid;
  place-items: center;
  border: 0;
  border-radius: 6px;
  color: rgba(255, 255, 255, 0.82);
  background: transparent;
  cursor: pointer;
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.dl-btn:hover,
.dl-btn.active {
  color: #fff;
  background: rgba(255, 255, 255, 0.14);
}
.dl-btn.active {
  color: var(--color-primary, #4daf7c);
}
.dl-btn svg {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.text-btn {
  width: 32px;
  font-size: 11px;
  font-weight: 650;
  letter-spacing: -0.03em;
}
.text-btn.wide {
  width: 44px;
  font-size: 10px;
  font-weight: 550;
}

.dl-lyric {
  position: absolute;
  z-index: 1;
  inset: 0;
  min-width: 0;
  min-height: 0;
  opacity: var(--dl-text-opacity, 1);
  transition: opacity 0.2s ease;
}
.dl-root.gradient-bar::before {
  content: '';
  position: absolute;
  z-index: 0;
    inset: 16% 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(0, 0, 0, var(--dl-gradient-opacity, 0.52)) var(--dl-gradient-edge, 14%),
    rgba(0, 0, 0, var(--dl-gradient-opacity, 0.52)) calc(100% - var(--dl-gradient-edge, 14%)),
    transparent 100%
  );
  pointer-events: none;
}
.dl-lyric :deep([data-role='line-normal-text']),
.dl-lyric :deep([data-role='line-normal-annotation']),
.dl-lyric :deep([data-role='line-normal-text-word-roman']) {
  font-weight: var(--dl-font-weight, 600) !important;
  -webkit-text-stroke: var(--dl-stroke-width, 0) var(--dl-stroke-color, #151515);
  paint-order: stroke fill;
}
.dl-root.text-shadow .dl-lyric :deep([data-role='line-normal-text']),
.dl-root.text-shadow .dl-lyric :deep([data-role='line-normal-annotation']),
.dl-root.text-shadow .dl-lyric :deep([data-role='line-normal-text-word-roman']) {
  text-shadow: var(--dl-shadow-x, 0px) var(--dl-shadow-y, 2px)
    var(--dl-shadow-blur, 6px) rgba(0, 0, 0, var(--dl-shadow-opacity, 0.72));
}
.dl-lyric.hidden {
  visibility: hidden;
  pointer-events: none;
}
.dl-lyric :deep([data-role='line-normal']) {
  row-gap: 10px !important;
  max-width: calc(100vw - 36px);
}
.dl-lyric :deep([data-role='line-normal-text-word-roman']) {
  white-space: nowrap;
  pointer-events: none;
  font-weight: var(--dl-font-weight, 600) !important;
}
.dl-lyric :deep([data-role='line-normal-annotation-translation']),
.dl-lyric :deep([data-role='line-normal-annotation-romanization']) {
  display: block !important;
  position: relative !important;
  line-height: 1.35;
  margin: 0;
  font-weight: var(--dl-font-weight, 600) !important;
}
.bold-syllable
  .dl-lyric
  :deep([data-role='line-normal-text']:has([data-role='line-normal-text-word'])) {
  font-weight: max(700, var(--dl-font-weight, 600)) !important;
}
.bold-line
  .dl-lyric
  :deep([data-role='line-normal-text']:not(:has([data-role='line-normal-text-word']))) {
  font-weight: max(700, var(--dl-font-weight, 600)) !important;
}
.bold-extended .dl-lyric :deep([data-role='line-normal-annotation']),
.bold-extended .dl-lyric :deep([data-role='line-normal-text-word-roman']) {
  font-weight: max(650, var(--dl-font-weight, 600)) !important;
}
.ellipsis .dl-lyric :deep([data-role='line-normal-text']),
.ellipsis .dl-lyric :deep([data-role='line-normal-annotation']) {
  display: block !important;
  max-width: calc(100vw - 44px);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.direction-vertical .dl-lyric :deep([data-role='line-normal']) {
  max-height: calc(100vh - 28px);
  writing-mode: vertical-rl;
  text-orientation: mixed;
}

.dl-placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 5px;
  color: rgba(255, 255, 255, 0.72);
}
.dl-placeholder span {
  font-size: 17px;
  font-weight: 650;
}
.dl-placeholder small {
  font-size: 9px;
  letter-spacing: 0.22em;
  opacity: 0.45;
}

.dl-visualizer {
  position: absolute;
  z-index: 2;
  left: 50%;
  bottom: 9px;
  width: min(52%, 310px);
  height: 18px;
  display: flex;
  align-items: end;
  justify-content: center;
  gap: 3px;
  transform: translateX(-50%);
  opacity: 0.3;
  pointer-events: none;
}
.dl-visualizer i {
  width: 3px;
  height: calc(2px + var(--bar-level, 0) * 16px);
  border-radius: 999px;
  background: currentColor;
  opacity: calc(0.28 + var(--bar-level, 0) * 0.72);
  transition: opacity var(--anim-dur-fast) linear;
}
.dl-visualizer.paused i {
  height: 2px;
  opacity: 0.24;
}

@media (prefers-reduced-motion: reduce) {
  .dl-root,
  .dl-toolbar,
  .dl-btn,
  .dl-lyric {
    transition: none;
  }
  .dl-visualizer i {
    transition: none;
  }
}
</style>
