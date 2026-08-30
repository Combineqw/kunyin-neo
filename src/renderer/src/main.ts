/**
 * 主窗口渲染进程入口。
 * 在 Vue 挂载前完成首屏主题、平台标记和最近路由恢复，随后由 App.vue 编排窗口级订阅。
 */
// 必须最先注册：Vue 与路由模块初始化前即可将首屏异常写入 fallback，且不放宽 CSP。
import './startupDiagnostics'
import './assets/main.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import type { AppSettings } from '@common'
import App from './App.vue'
import { router, getLastRoute } from './router'
import { applyTheme, initTheme, setComfortLevel, setCustomThemes } from './theme/apply'
import { applyAppFont } from './composables/useFonts'
import { installPressFeedback } from './composables/usePressFeedback'
import { applyAnimationPack } from './animation/apply'
import { DEFAULT_CUSTOM_ANIMATION_PACK } from '@common'

// OS class + 语言（驱动字体栈），并同步注入主题变量（首屏无闪烁）。
// preload 已同步读取 settings.json 的外观段；读不到时回退默认绿色。
const platform = navigator.platform.toLowerCase()
document.documentElement.classList.add(
  platform.includes('mac') ? 'mac' : platform.includes('linux') ? 'linux' : 'windows'
)
document.documentElement.lang = 'zh-Hans'

const initialAppearance = (
  window as unknown as { __INITIAL_APPEARANCE__?: AppSettings['appearance'] | null }
).__INITIAL_APPEARANCE__
if (initialAppearance) {
  setCustomThemes(initialAppearance.customThemes ?? [])
  // 首屏就带上亮度档位，否则窗口可见后才变暗，会看到一次闪白
  setComfortLevel(initialAppearance.comfortLevel ?? 'standard')
  applyTheme(
    initialAppearance.themeId ?? 'green',
    initialAppearance.lightThemeId ?? 'green',
    initialAppearance.darkThemeId ?? 'black'
  )
  // 缩放/字体也同步注入，避免 settings.load 回来后窗口已可见再发生布局跳变
  document.documentElement.style.setProperty(
    '--app-zoom',
    String((initialAppearance.fontSize ?? 16) / 16)
  )
  applyAppFont(initialAppearance.appFont ?? '')
  applyAnimationPack('ios', { ...DEFAULT_CUSTOM_ANIMATION_PACK })
} else {
  initTheme('green')
  applyAnimationPack('ios', { ...DEFAULT_CUSTOM_ANIMATION_PACK })
}

// 按压弹回的全局委托监听（3 个 window 监听器，不随元件数量增长）
installPressFeedback()

const app = createApp(App).use(createPinia()).use(router)

// 恢复上次停留的页面（在挂载前排队导航，router.isReady 会以它为准）
const lastRoute = getLastRoute()
if (lastRoute) void router.replace(lastRoute)
app.mount('#app')
document.getElementById('app')?.setAttribute('data-mounted', 'true')
