<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import AppIcon from './AppIcon.vue'
import { useSearchStore } from '../stores/search'
import { useApi } from '../composables/useApi'

const api = useApi()
const router = useRouter()
const searchStore = useSearchStore()
const { keyword } = storeToRefs(searchStore)
const local = ref('')
const isFullscreen = ref(false)
let unsubscribeFullscreen: (() => void) | null = null

const focused = ref(false)
const tips = ref<string[]>([])
let tipTimer: ReturnType<typeof setTimeout> | null = null

function go(q: string): void {
  const kw = q.trim()
  if (!kw) return
  local.value = kw
  keyword.value = kw
  focused.value = false
  router.push({ name: 'search' })
  void searchStore.search()
}

function submit(): void {
  go(local.value)
}

function onInput(): void {
  if (tipTimer) clearTimeout(tipTimer)
  const q = local.value.trim()
  if (!q) {
    tips.value = []
    return
  }
  tipTimer = setTimeout(async () => {
    tips.value = await searchStore.tips(q)
  }, 250)
}

function onBlur(): void {
  // 延迟收起，保证点击下拉项能触发
  setTimeout(() => (focused.value = false), 150)
}

function clearInput(): void {
  local.value = ''
  tips.value = []
  // 彻底清除当前搜索:关键词 + 结果一并清空,搜索页回到历史/热搜
  keyword.value = ''
  searchStore.clearResults()
}

function toggleFullscreen(): void {
  void api.window.fullscreen(!isFullscreen.value).then((value) => {
    isFullscreen.value = value
  })
}

onMounted(() => {
  void api.window.fullscreen().then((value) => {
    isFullscreen.value = value
  })
  unsubscribeFullscreen = api.window.onFullscreenChange((value) => {
    isFullscreen.value = value
  })
})

onUnmounted(() => unsubscribeFullscreen?.())
</script>

<template>
  <header class="toolbar drag">
    <div class="search-wrap no-drag">
      <div class="search-box" :class="{ open: focused }">
        <div class="form">
          <AppIcon name="search" :size="15" />
          <input
            v-model="local"
            class="search-input"
            type="text"
            placeholder="搜索歌曲、歌手、专辑"
            spellcheck="false"
            @focus="focused = true"
            @blur="onBlur"
            @input="onInput"
            @keyup.enter="submit"
          />
          <button v-if="local" class="clear pressable" title="清空" @click="clearInput">
            <AppIcon name="close" :size="13" />
          </button>
        </div>
        <!-- 搜索提示（LX SearchInput：与输入框同盒向下展开，无独立浮层） -->
        <ul v-if="focused && local && tips.length" class="dropdown">
          <li
            v-for="t in tips"
            :key="t"
            class="dd-item pressable pressable-subtle"
            @mousedown.prevent="go(t)"
          >
            <span class="ellipsis-2">{{ t }}</span>
          </li>
        </ul>
      </div>
    </div>

    <div class="win-controls no-drag">
      <button class="win-btn min pressable" title="最小化" @click="api.window.minimize()">
        <AppIcon name="minus" :size="16" />
      </button>
      <button
        class="win-btn fullscreen pressable"
        :title="isFullscreen ? '退出全屏' : '全屏'"
        @click="toggleFullscreen"
      >
        <AppIcon :name="isFullscreen ? 'restore' : 'maximize'" :size="15" />
      </button>
      <button class="win-btn close pressable" title="关闭" @click="api.window.close()">
        <AppIcon name="close" :size="16" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.toolbar {
  flex: none;
  height: var(--height-toolbar);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 10px 0 16px;
}
.search-wrap {
  position: relative;
  width: 36%;
  min-width: 250px;
  height: 32px;
}
/* Apple HIG 胶囊搜索框与毛玻璃下拉菜单 */
.search-box {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 50;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--color-font) 10%, transparent);
  color: var(--color-button-font);
  background-color: color-mix(in srgb, var(--color-main-background) 65%, transparent);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  transition:
    background-color var(--anim-dur-fast) var(--anim-ease-smooth),
    box-shadow var(--anim-dur-fast) var(--anim-ease-smooth),
    border-color var(--anim-dur-fast) var(--anim-ease-smooth);
}
.search-box:hover,
.search-box.open {
  background-color: color-mix(in srgb, var(--color-main-background) 88%, transparent);
  border-color: color-mix(in srgb, var(--color-primary) 35%, transparent);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.08);
}
.search-box.open {
  border-radius: 16px;
  overflow: hidden;
}
.form {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  padding: 0 12px;
}
.search-input {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  letter-spacing: -0.01em;
  color: var(--color-font);
  background: none;
}
.search-input::placeholder {
  color: var(--color-font-label);
}
.clear {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  color: var(--color-font-label);
  transition:
    color var(--anim-dur-fast) var(--anim-ease-smooth),
    background-color var(--anim-dur-fast) var(--anim-ease-smooth);
}
.clear:hover {
  color: var(--color-font);
  background-color: color-mix(in srgb, var(--color-font) 8%, transparent);
}

/* 提示列表：在搜索盒内（同背景同圆角），行 hover 用主题深色半透明（LX .select） */
.dropdown {
  max-height: 320px;
  overflow-y: auto;
  padding: 4px 6px 6px;
  font-size: 13px;
}
.dd-item {
  cursor: pointer;
  padding: 8px 12px;
  line-height: 1.3;
  color: var(--color-font);
  border-radius: 10px;
  transition: background-color var(--anim-dur-fast) var(--anim-ease-smooth);
}
.dd-item:hover {
  background-color: var(--color-primary-light-300-alpha-600);
}

/* 窗口控制按钮：精致圆角矩形，hover 主题色 */
.win-controls {
  flex: none;
  display: flex;
  align-items: center;
  gap: 4px;
}
.win-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 28px;
  border-radius: 8px;
  color: var(--color-font-label);
  transition:
    color var(--anim-dur-fast) var(--anim-ease-smooth),
    background-color var(--anim-dur-fast) var(--anim-ease-smooth);
}
.win-btn:hover {
  color: #fff;
}
.win-btn.min:hover {
  background-color: color-mix(in srgb, var(--color-btn-min) 80%, transparent);
}
.win-btn.fullscreen:hover {
  background-color: color-mix(in srgb, var(--color-btn-hide) 80%, transparent);
}
.win-btn.close:hover {
  background-color: color-mix(in srgb, var(--color-btn-close) 80%, transparent);
}
</style>
