<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { RouterLink, useRoute } from 'vue-router'
import AppIcon from './AppIcon.vue'

// 结构与交互移植自 lx-music-desktop（Apache-2.0, © lyswhut）
// components/layout/Aside/{index.vue, NavBar.vue}：文字 logo + 满宽近方形导航项。
const navItems = [
  { name: 'search', label: '搜索', icon: 'search' },
  { name: 'recommendations', label: '每日推荐与心动模式', icon: 'heart-mode' },
  { name: 'playlists', label: '歌单', icon: 'library' },
  { name: 'download', label: '下载', icon: 'download' },
  { name: 'plugin-store', label: '插件商店', icon: 'plus' },
  { name: 'settings', label: '设置', icon: 'settings' }
] as const

const route = useRoute()
const listRef = ref<HTMLElement>()
const indicatorY = ref(0)
const indicatorHeight = ref(0)

function updateIndicator(): void {
  const list = listRef.value
  const active = list?.querySelector<HTMLElement>('.link.active')
  const activeItem = active?.closest<HTMLElement>('.nav-item')
  if (!list || !activeItem) {
    indicatorHeight.value = 0
    return
  }
  indicatorY.value = activeItem.offsetTop
  indicatorHeight.value = activeItem.offsetHeight
}

watch(() => route.fullPath, () => void nextTick(updateIndicator))
onMounted(() => {
  updateIndicator()
  window.addEventListener('resize', updateIndicator)
})
onUnmounted(() => window.removeEventListener('resize', updateIndicator))
</script>

<template>
  <aside class="aside">
    <nav class="menu">
      <ul ref="listRef" class="list" role="toolbar">
        <span
          class="active-indicator"
          :style="{ transform: `translate3d(0, ${indicatorY}px, 0)`, height: `${indicatorHeight}px` }"
          aria-hidden="true"
        />
        <li v-for="item in navItems" :key="item.name" class="nav-item" role="presentation">
          <RouterLink
            :to="{ name: item.name }"
            class="link no-drag pressable pressable-subtle"
            active-class="active"
            role="tab"
            :aria-label="item.label"
            :title="item.label"
          >
            <AppIcon :name="item.icon" :size="20" />
          </RouterLink>
        </li>
      </ul>
    </nav>
  </aside>
</template>

<style scoped>
/* 整栏可拖动窗口；nav 项 84% padding 撑起近方形，active 左侧竖条全高滑入（LX 原版效果） */
.aside {
  height: 100%;
  display: flex;
  flex-flow: column nowrap;
  -webkit-app-region: drag;
  -webkit-user-select: none;
}
.logo {
  flex: none;
  box-sizing: border-box;
  padding: 0 13%;
  height: 50px;
  line-height: 50px;
  text-align: center;
  font-weight: bold;
  font-size: 15px;
  color: var(--color-nav-font);
  opacity: 0.8;
  overflow: hidden;
  white-space: nowrap;
}
.menu {
  flex: auto;
}
.list {
  position: relative;
  -webkit-app-region: no-drag;
}
.active-indicator {
  position: absolute;
  z-index: 1;
  left: 0;
  width: 3px;
  border-radius: 0 4px 4px 0;
  pointer-events: none;
  background-color: var(--color-primary-dark-200-alpha-700);
  transition:
    transform var(--anim-dur-base) var(--anim-ease-spring),
    opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.nav-item {
  position: relative;
}
.nav-item::before {
  content: '';
  display: block;
  width: 100%;
  padding-bottom: 84%;
}
.link {
  position: absolute;
  left: 0;
  top: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-nav-font);
  cursor: pointer;
  outline: none;
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.link.active {
  background-color: var(--color-primary-light-300-alpha-700);
}
.link.active:hover {
  background-color: var(--color-primary-light-300-alpha-800);
}
.link:hover {
  color: var(--color-nav-font);
}
.link:hover:not(.active) {
  opacity: 0.8;
  background-color: var(--color-primary-light-400-alpha-700);
}
.link:active:not(.active) {
  opacity: 0.6;
  background-color: var(--color-primary-light-300-alpha-600);
}
</style>
