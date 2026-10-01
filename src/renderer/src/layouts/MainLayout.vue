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
    <div class="aurora-backdrop" aria-hidden="true" />
    <AppAside class="area-aside glass-surface glass-thin" />
    <div class="area-main glass-surface glass-regular">
      <AppToolbar class="glass-thin" />
      <main class="view scroll">
        <div class="view-content">
          <RouterView v-slot="{ Component, route: viewRoute }">
            <Transition name="page-slide" mode="out-in">
              <KeepAlive :include="KEEP_ALIVE">
                <component :is="Component" :key="viewRoute.fullPath" />
              </KeepAlive>
            </Transition>
          </RouterView>
        </div>
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
  position: relative;
  display: flex;
  height: 100%;
  gap: 10px;
  padding: 10px;
  box-sizing: border-box;
  background-color: var(--color-app-background);
  isolation: isolate;
}
.aurora-backdrop {
  position: absolute;
  z-index: 0;
  inset: 0;
  pointer-events: none;
  opacity: 0;
  background:
    radial-gradient(ellipse 70% 62% at 10% 5%, color-mix(in srgb, var(--aurora-c1) 86%, transparent), transparent 70%),
    radial-gradient(ellipse 68% 58% at 92% 12%, color-mix(in srgb, var(--aurora-c3) 82%, transparent), transparent 70%),
    radial-gradient(ellipse 80% 65% at 45% 100%, color-mix(in srgb, var(--aurora-c1) 58%, transparent), transparent 74%);
  animation: aurora-backdrop-drift var(--anim-dur-aurora) ease-in-out infinite alternate;
  transition: opacity var(--anim-dur-theme) var(--anim-ease-smooth);
}
html[data-theme^='aurora'] .aurora-backdrop {
  opacity: 0.86;
}
html.theme-dark[data-theme^='aurora'] .aurora-backdrop {
  opacity: 0.48;
}
@keyframes aurora-backdrop-drift {
  from { transform: scale(1) translate3d(-1%, -1%, 0); }
  to { transform: scale(1.08) translate3d(1%, 1%, 0); }
}
.shell > *:not(.aurora-backdrop) {
  position: relative;
  z-index: 1;
}
.area-aside {
  flex: none;
  width: var(--width-aside);
  height: 100%;
  border-radius: 18px;
  overflow: hidden;
}
.area-main {
  flex: 1;
  min-height: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background-color: var(--glass-regular-background);
  border-radius: 22px;
  box-shadow: var(--glass-shadow);
  border: 1px solid var(--glass-border);
  overflow: hidden;
  /* The main surface covers most of the window. Keep its glass depth subtle so
     scrolling does not repeatedly re-rasterize a large backdrop-filter region. */
  --glass-regular-blur: 8px;
  --glass-regular-saturation: 1.08;
}
.view {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  /* Keep a stable vertical track so detail pages expose their scroll affordance
     even while async playlist rows are still being populated. */
  overflow-y: scroll;
  scrollbar-gutter: stable;
  overscroll-behavior: contain;
}
html[data-theme^='aurora'] .area-main {
  background: color-mix(in srgb, var(--color-main-background) 44%, transparent);
}
html[data-theme^='aurora'] .area-aside {
  background: color-mix(in srgb, var(--color-main-background) 44%, transparent);
}
html[data-theme^='aurora'] .view {
  background: transparent;
}
.view-content {
  /* Keep the routed page's intrinsic height in the scroll container. A bare
     RouterView/Transition is not a flex item, so its page can otherwise be
     measured against the shrinking .view during route changes. */
  flex: 0 0 auto;
  min-height: 100%;
  width: 100%;
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

@media (prefers-reduced-motion: reduce), (max-width: 900px) {
  .aurora-backdrop {
    animation: none;
  }
}
</style>
