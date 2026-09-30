<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import AppAside from '../components/AppAside.vue'
import AppToolbar from '../components/AppToolbar.vue'
import PlayerBar from '../components/PlayerBar.vue'

const KEEP_ALIVE = [
  'SearchView',
  'RecommendationsView',
  'PlaylistsView',
  'DownloadView',
  'SettingsView'
]
const route = useRoute()
const wipeKey = ref('')
let wipeTimer: number | undefined

watch(
  () => route.fullPath,
  (path, previousPath) => {
    if (!previousPath) return
    wipeKey.value = `${path}:${Date.now()}`
    window.clearTimeout(wipeTimer)
    void nextTick(() => {
      wipeTimer = window.setTimeout(() => {
        wipeKey.value = ''
      }, 420)
    })
  }
)
</script>

<template>
  <div class="shell">
    <AppAside class="area-aside glass-surface glass-thin" />
    <div class="area-main glass-surface glass-regular">
      <AppToolbar class="glass-thin" />
      <main class="view scroll">
        <RouterView v-slot="{ Component, route: viewRoute }">
          <Transition name="page-slide" mode="out-in">
            <KeepAlive :include="KEEP_ALIVE">
              <component :is="Component" :key="viewRoute.fullPath" />
            </KeepAlive>
          </Transition>
        </RouterView>
        <span v-if="wipeKey" :key="wipeKey" class="route-wipe" aria-hidden="true" />
      </main>
      <PlayerBar class="glass-surface glass-thin" />
    </div>
  </div>
</template>

<style scoped>
/* 窄图标侧栏（透出浅色 app-background）| 白色主区（LX 布局） */
/* 高度用 100%（跟随被 zoom 缩放的 #app），不能用 100vh——vh 是视口单位不随 zoom 缩放，
   字体大小档位改变 zoom 后会与 #app 高度失配，导致底部露白。 */
.shell {
  display: flex;
  height: 100%;
  gap: 10px;
  padding: 10px;
  box-sizing: border-box;
  background-color: var(--color-app-background);
}
.area-aside {
  flex: none;
  width: var(--width-aside);
  height: 100%;
}
.area-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background-color: var(--glass-regular-background);
  border-radius: 22px;
  box-shadow: var(--glass-shadow);
  border: 1px solid var(--glass-border);
  overflow: hidden;
}
.view {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.page-slide-enter-active,
.page-slide-leave-active {
  transition:
    transform var(--anim-dur-base) var(--anim-ease-standard),
    opacity var(--anim-dur-fast) var(--anim-ease-smooth);
}
.page-slide-enter-from {
  opacity: 0;
  transform: translate3d(20px, 0, 0);
}
.page-slide-leave-to {
  opacity: 0;
  transform: translate3d(-12px, 0, 0);
}

.route-wipe {
  position: absolute;
  z-index: 2;
  inset: 0;
  pointer-events: none;
  background-color: var(--color-main-background);
  transform: translate3d(-102%, 0, 0);
  animation: route-wipe var(--anim-dur-base) var(--anim-ease-smooth) both;
}

@keyframes route-wipe {
  0% {
    opacity: 0.82;
    transform: translate3d(-102%, 0, 0);
  }
  48% {
    opacity: 0.14;
    transform: translate3d(0, 0, 0);
  }
  100% {
    opacity: 0;
    transform: translate3d(102%, 0, 0);
  }
}
</style>
