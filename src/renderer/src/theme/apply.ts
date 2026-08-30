// 运行期主题注入。移植自 lx-music-desktop 的 setTheme/applyTheme 思路：
// 主题变量写入 <style> 的 :root 块；'auto' 时按系统深浅色在 light/dark 两主题间切换。
import {
  buildThemeColors,
  customToThemeDef,
  findTheme,
  THEMES,
  type CustomThemeConfig,
  type ThemeDef
} from './themes'
import { applyComfort, type ComfortLevel } from './comfort'

const STYLE_ID = 'theme-vars'

function setThemeVars(colors: Record<string, string>): void {
  let el = document.getElementById(STYLE_ID) as HTMLStyleElement | null
  const body = Object.entries(colors)
    .map(([k, v]) => `${k}:${v};`)
    .join('')
  if (el) {
    // 幂等：内容没变就不重写 textContent。重写会让 --background-image 的 url()
    // 被移除再解析，背景图短暂消失再出现（启动时二次 applyTheme 会触发）。
    if (el.textContent === `:root{${body}}`) return
  } else {
    el = document.createElement('style')
    el.id = STYLE_ID
    document.head.appendChild(el)
  }
  el.textContent = `:root{${body}}`
}

const darkQuery = window.matchMedia('(prefers-color-scheme: dark)')

let currentId = 'green'
let currentLightId = 'green'
let currentDarkId = 'black'
let customThemes: ThemeDef[] = []
let comfortLevel: ComfortLevel = 'standard'

function resolve(id: string): ThemeDef {
  const themeId = id === 'auto' ? (darkQuery.matches ? currentDarkId : currentLightId) : id
  return (
    findTheme(themeId, customThemes) ??
    // 兜底：引用了不存在的主题（如已删除的自定义主题）时回落到默认深浅主题
    findTheme(darkQuery.matches ? 'black' : 'green', customThemes) ??
    THEMES[0]
  )
}

/** 应用主题。id 可为具体主题 id 或 'auto'（跟随系统深浅）。 */
export function applyTheme(id: string, lightId = currentLightId, darkId = currentDarkId): void {
  currentId = id
  currentLightId = lightId
  currentDarkId = darkId
  const theme = resolve(id)
  // 亮度档位在最后一步统一后处理，覆盖内置与自定义主题
  setThemeVars(applyComfort(buildThemeColors(theme), comfortLevel, theme.isDark))
  document.documentElement.classList.toggle('theme-dark', theme.isDark)
  // 已解析的具体主题 id（'auto' 已在 resolve 里落到真实主题）。
  // CSS 侧据此把极光装饰限定在 aurora_* 五个主题上：html[data-theme^='aurora']。
  // 写具体 id 而不是写一个 has-aurora 布尔类，是为了让将来按单个主题微调也够用。
  document.documentElement.dataset.theme = theme.id
}

/** 记录界面亮度档位。只写状态不重放——调用方紧接着会 applyTheme，
 * 在这里再调一次会让启动时整张色表算两遍。与 setCustomThemes 的约定一致。 */
export function setComfortLevel(level: ComfortLevel): void {
  comfortLevel = level
}

/** 同步用户自定义主题列表（settings.appearance.customThemes 变更时调用） */
export function setCustomThemes(configs: CustomThemeConfig[]): void {
  customThemes = configs.map(customToThemeDef)
}

// 系统深浅变化时，若处于 auto 则重新应用
darkQuery.addEventListener('change', () => {
  if (currentId === 'auto') applyTheme('auto')
})

/** 首屏初始化（同步注入默认主题变量，避免无样式闪烁） */
export function initTheme(id = 'green'): void {
  applyTheme(id)
}

/** 当前已解析主题的背景图 URL（无则 null）。
 * 启动流程在显示窗口前预载它，避免窗口可见后图片才解码弹入。 */
export function currentBackgroundImageUrl(): string | null {
  const v = resolve(currentId).ext['--background-image']
  if (!v || v === 'none') return null
  const m = /^url\((?:"|')?(.*?)(?:"|')?\)$/.exec(v.trim())
  return m?.[1] || null
}
