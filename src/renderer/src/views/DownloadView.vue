<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import AppIcon from '../components/AppIcon.vue'
import { useDownloadStore } from '../stores/download'
import { coverUrl } from '../utils/cover'
import type { DownloadStatus } from '@common'

defineOptions({ name: 'DownloadView' })

const download = useDownloadStore()
const { tasks } = storeToRefs(download)

const hasCompleted = computed(() => tasks.value.some((t) => t.status === 'completed'))

function fmtSize(bytes: number): string {
  if (!bytes) return ''
  const mb = bytes / 1024 / 1024
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${(bytes / 1024).toFixed(0)} KB`
}
function fmtSpeed(bps: number): string {
  if (!bps) return ''
  const mb = bps / 1024 / 1024
  return mb >= 1 ? `${mb.toFixed(1)} MB/s` : `${(bps / 1024).toFixed(0)} KB/s`
}
function statusText(s: DownloadStatus): string {
  return {
    waiting: '等待中',
    downloading: '下载中',
    paused: '已暂停',
    completed: '已完成',
    failed: '失败'
  }[s]
}
</script>

<template>
  <div v-if="tasks.length" class="page">
    <div class="head">
      <span class="title">下载任务（{{ tasks.length }}）</span>
      <button v-if="hasCompleted" class="clear pressable" @click="download.clearCompleted()">
        清除已完成
      </button>
    </div>
    <TransitionGroup name="download-task" tag="div" class="list">
      <div
        v-for="(t, i) in tasks"
        :key="t.taskKey"
        class="task"
        :class="t.status"
        :style="{ '--download-delay': `${Math.min(i, 20) * 30}ms` }"
      >
        <div class="cover">
          <img v-if="t.cover" :src="coverUrl(t.cover)" alt="" />
          <AppIcon v-else name="download" :size="18" />
        </div>
        <div class="meta">
          <div class="title-row">
            <span class="song-title ellipsis">{{ t.title }}</span>
            <span class="quality">{{ t.qualityName }}</span>
          </div>
          <div class="artist ellipsis">{{ t.artist }}</div>
          <div class="progress-bar">
            <div
              class="progress-fill"
              :style="{ transform: `scaleX(${t.progress})`, '--fill-scale': t.progress }"
            >
              <span v-if="t.status === 'downloading'" class="fill-aurora aurora-band" />
            </div>
          </div>
          <div class="sub">
            <span class="status">{{ statusText(t.status) }}</span>
            <span v-if="t.status === 'completed'" class="done-mark" aria-label="已完成">
              <AppIcon name="check" :size="13" />
            </span>
            <span v-if="t.status === 'downloading'" class="detail">
              {{ fmtSpeed(t.speedBytesPerSec) }} · {{ fmtSize(t.downloadedBytes) }} /
              {{ fmtSize(t.totalBytes) }}
            </span>
            <span v-else-if="t.status === 'failed'" class="err ellipsis">{{ t.errorMessage }}</span>
          </div>
        </div>
        <div class="ops">
          <button
            v-if="t.status === 'downloading' || t.status === 'waiting'"
            class="op pressable"
            title="暂停"
            @click="download.pause(t.taskKey)"
          >
            <AppIcon name="pause" :size="16" />
          </button>
          <button
            v-else-if="t.status === 'paused'"
            class="op pressable"
            title="继续"
            @click="download.resume(t.taskKey)"
          >
            <AppIcon name="play" :size="16" />
          </button>
          <button
            v-else-if="t.status === 'failed'"
            class="op pressable"
            title="重试"
            @click="download.retry(t.taskKey)"
          >
            <AppIcon name="download" :size="16" />
          </button>
          <button class="op pressable" title="移除" @click="download.remove(t.taskKey)">
            <AppIcon name="trash" :size="16" />
          </button>
        </div>
      </div>
    </TransitionGroup>
  </div>

  <div v-else class="empty">
    <span class="empty-icon"><AppIcon name="download" :size="40" /></span>
    <p class="empty-text">暂无下载任务</p>
    <p class="empty-hint">在歌曲上选择下载，即可离线收听</p>
  </div>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}
.title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-font);
}
.clear {
  font-size: 12px;
  color: var(--color-font-label);
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.clear:hover {
  color: var(--color-primary-font);
}
.list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.task {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px;
  border-radius: var(--radius-border);
  background: var(--color-primary-background);
}
.download-task-enter-active {
  animation: download-task-in var(--anim-dur-base) var(--anim-ease-standard) both;
  animation-delay: var(--download-delay, 0ms);
}
.download-task-leave-active {
  position: absolute;
  width: 100%;
  opacity: 0;
  transform: translate3d(18px, 0, 0);
  transition:
    transform var(--anim-dur-fast) var(--anim-ease-standard),
    opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.download-task-move {
  transition: transform var(--anim-dur-fast) var(--anim-ease-smooth);
}
@keyframes download-task-in {
  from {
    opacity: 0;
    transform: translate3d(12px, 0, 0);
  }
  to {
    opacity: 1;
    transform: translate3d(0, 0, 0);
  }
}
.cover {
  flex: none;
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-border);
  overflow: hidden;
  color: var(--color-font-label);
  background: var(--color-primary-background-hover);
}
.cover img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.meta {
  flex: 1;
  min-width: 0;
}
.title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.song-title {
  font-size: 13px;
  color: var(--color-font);
}
.quality {
  flex: none;
  font-size: 11px;
  color: var(--color-primary-font);
}
.artist {
  font-size: 12px;
  color: var(--color-font-label);
}
.progress-bar {
  height: 3px;
  margin: 6px 0 4px;
  border-radius: 999px;
  background: var(--color-primary-background-hover);
}
.progress-fill {
  position: relative;
  height: 100%;
  border-radius: 999px;
  /* 光带子层用反向 scaleX 抵消这里的压缩，故需裁掉超出填充宽度的部分 */
  overflow: hidden;
  transform-origin: left center;
  background: var(--color-primary);
  transition: transform var(--anim-dur-fast) linear;
}
/* 极光流光：填充层用 scaleX 表示百分比，会把子元素横向压扁，因此这里用
   1/scale 反向拉伸，使渐变始终以 1:1 真实比例呈现（低百分比时也不糊）。
   父层 overflow:hidden 在本地坐标系裁切，等效于"进度推进时逐段揭开光带"。
   --fill-scale 为 0 时 calc 会除零 → 用 max() 兜底，避免 NaN 导致整层消失。 */
.fill-aurora {
  position: absolute;
  inset: 0;
  pointer-events: none;
  transform-origin: left center;
  transform: scaleX(calc(1 / max(var(--fill-scale, 1), 0.001)));
  /* 下载场景需要活性，比全局默认 7s 更快 */
  --anim-dur-aurora: 5s;
}
.done-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  color: var(--color-main-background);
  background-color: var(--color-primary);
  animation: done-pop var(--anim-dur-base) var(--anim-ease-spring) both;
}
@keyframes done-pop {
  from {
    opacity: 0;
    transform: scale(0.5);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
.task.completed .progress-fill {
  background: var(--color-primary);
}
.task.failed .progress-fill {
  background: #e05a5a;
}
.sub {
  display: flex;
  gap: 10px;
  font-size: 11px;
  color: var(--color-font-label);
}
.err {
  color: #e05a5a;
}
.ops {
  flex: none;
  display: flex;
  gap: 6px;
}
.op {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  color: var(--color-font-label);
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.op:hover {
  color: var(--color-primary-font);
  background: var(--color-primary-background-hover);
}

.empty {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
}
.empty-icon {
  display: flex;
  color: var(--color-font-label);
  opacity: 0.5;
  margin-bottom: 4px;
}
.empty-text {
  font-size: 15px;
  color: var(--color-font);
}
.empty-hint {
  font-size: 13px;
  color: var(--color-font-label);
}
</style>
