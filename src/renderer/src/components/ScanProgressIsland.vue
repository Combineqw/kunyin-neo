<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { LibraryScanProgress } from '@common'
import { useApi } from '../composables/useApi'

const api = useApi()
const progress = ref<LibraryScanProgress | null>(null)
const percent = computed(() => {
  const current = progress.value
  return !current || current.total <= 0
    ? 0
    : Math.min(100, Math.round((current.done / current.total) * 100))
})
let unsubscribe: (() => void) | undefined
let dismissTimer: number | undefined

function onProgress(next: LibraryScanProgress): void {
  progress.value = next
  window.clearTimeout(dismissTimer)
  if (next.phase === 'done' || next.phase === 'cancelled' || next.phase === 'error') {
    dismissTimer = window.setTimeout(() => {
      progress.value = null
    }, 2200)
  }
}

function cancel(): void {
  const taskId = progress.value?.taskId
  if (taskId) void api.library.cancelScan(taskId)
}

function isActive(): boolean {
  return (
    progress.value?.phase === 'collecting' ||
    progress.value?.phase === 'scanning' ||
    progress.value?.phase === 'enriching'
  )
}

onMounted(() => {
  unsubscribe = api.library.onScanProgress(onProgress)
})

onUnmounted(() => {
  unsubscribe?.()
  window.clearTimeout(dismissTimer)
})
</script>

<template>
  <Transition name="scan-island">
    <div v-if="progress" class="scan-island" role="status">
      <div class="scan-island-head">
        <span>
          {{
            progress.taskKind === 'enrich' && progress.phase === 'enriching'
              ? `正在补全本地信息 ${progress.done}/${progress.total}`
              : progress.phase === 'collecting'
                ? '准备扫描本地音乐'
                : progress.phase === 'committing'
                  ? '正在写入曲库'
                  : progress.phase === 'done'
                    ? progress.taskKind === 'enrich'
                      ? `信息补全完成，更新 ${progress.added} 首`
                      : `扫描完成，已添加 ${progress.added} 首`
                    : progress.phase === 'cancelled'
                      ? '扫描已取消'
                      : progress.phase === 'error'
                        ? `扫描失败：${progress.error ?? '未知错误'}`
                        : `正在扫描 ${progress.done}/${progress.total}`
          }}
        </span>
        <button
          v-if="isActive()"
          type="button"
          :title="progress.taskKind === 'enrich' ? '取消补全' : '取消扫描'"
          @click="cancel"
        >
          取消
        </button>
      </div>
      <div
        v-if="
          progress.phase === 'collecting' ||
          progress.phase === 'scanning' ||
          progress.phase === 'enriching' ||
          progress.phase === 'committing'
        "
        class="scan-track"
      >
        <span
          class="scan-fill"
          :style="{ transform: `scaleX(${progress.phase === 'committing' ? 1 : percent / 100})` }"
        />
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.scan-island {
  position: fixed;
  right: 18px;
  bottom: calc(var(--height-player) + 18px);
  z-index: 90;
  width: min(360px, calc(100vw - 36px));
  padding: 10px 12px;
  border: 1px solid var(--color-primary-light-100-alpha-700);
  border-radius: 8px;
  color: var(--color-font);
  background: var(--color-content-background);
  box-shadow: 0 6px 22px rgba(0, 0, 0, 0.16);
}
.scan-island-head {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
}
.scan-island-head span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.scan-island-head button {
  flex: none;
  padding: 3px 8px;
  border-radius: var(--form-radius);
  color: var(--color-primary-font);
  background: var(--color-primary-background);
}
.scan-track {
  height: 3px;
  margin-top: 8px;
  overflow: hidden;
  border-radius: 2px;
  background: var(--color-primary-background);
}
.scan-fill {
  display: block;
  width: 100%;
  height: 100%;
  transform-origin: left center;
  background: var(--color-primary);
  transition: transform var(--anim-dur-fast) linear;
}
.scan-island-enter-active,
.scan-island-leave-active {
  transition:
    opacity var(--anim-dur-fast) var(--anim-ease-smooth),
    transform var(--anim-dur-fast) var(--anim-ease-smooth);
}
.scan-island-enter-from,
.scan-island-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
