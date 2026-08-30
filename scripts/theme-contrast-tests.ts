// 主题配色验收：把「不达标不交付」这条要求做成可重复执行的门。
//
// 跑的是真实的 themes.ts + comfort.ts，不是另写的模型——曾经有一次浅色护眼档位
// 的 AA 回归，自写模型验证放过了，只有真实代码路径才抓出来。
//
// 卡两头，缺一不可：
//   下限 AA   在用字档 ≥ 4.5:1。只卡下限会漏掉「亮得刺眼」。
//   上限 峰值  深色最亮档 ≤ 12.5:1。深色下峰值过高会光渗（halation），
//             即使饱和度为 0 也一样——这是三个深色主题被用户投诉的直接成因。
import assert from 'node:assert/strict'
import { THEMES, buildThemeColors, customToThemeDef } from '../src/renderer/src/theme/themes'
import { applyComfort } from '../src/renderer/src/theme/comfort'
import type { ComfortLevel, CustomThemeConfig } from '../src/common/types/settings'

/** WCAG AA 正文最低对比度 */
const AA = 4.5

/** 深色峰值上限。一般建议 ≤12:1，留 0.5 余量给取整。 */
const DARK_PEAK_MAX = 12.5

const LEVELS: ComfortLevel[] = ['standard', 'soft', 'softer']

/** 渲染层实际引用到的字档。全部 21 档里只有这些用于文字，其余是描边/分隔线等。
 *
 * 深色下 base.css 的 `html.theme-dark` 把 --color-font-label 从 --color-450
 * 覆盖到 --color-550，所以深色查 550、浅色查 lightExt 指定的 650。
 * 这条覆写在 CSS 里而不在 JS 色表里，必须手动分支，否则查错档位。 */
const TIERS_DARK = ['--color-1000', '--color-850', '--color-750', '--color-550']
const TIERS_LIGHT = ['--color-850', '--color-650']

type Rgb = { r: number; g: number; b: number; a: number }

function parseColor(value: string): Rgb | null {
  const m = /^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*(?:,\s*([\d.]+)\s*)?\)$/i.exec(
    value.trim()
  )
  if (m) return { r: +m[1], g: +m[2], b: +m[3], a: m[4] === undefined ? 1 : +m[4] }
  const h = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value.trim())
  if (!h) return null
  const hex =
    h[1].length === 3
      ? h[1]
          .split('')
          .map((c) => c + c)
          .join('')
      : h[1]
  return {
    r: parseInt(hex.slice(0, 2), 16),
    g: parseInt(hex.slice(2, 4), 16),
    b: parseInt(hex.slice(4, 6), 16),
    a: 1
  }
}

function relativeLuminance({ r, g, b }: Rgb): number {
  const ch = (c: number): number => {
    const s = c / 255
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
  }
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}

function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [relativeLuminance(a), relativeLuminance(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

/** 半透明前景压在背景上得到的真实像素。
 * 不做这一步就会把「卡片声明的颜色」当成「眼睛看到的颜色」，两者可以差好几倍。 */
function over(fg: Rgb, bg: Rgb): Rgb {
  return {
    r: fg.r * fg.a + bg.r * (1 - fg.a),
    g: fg.g * fg.a + bg.g * (1 - fg.a),
    b: fg.b * fg.a + bg.b * (1 - fg.a),
    a: 1
  }
}

/** 顺着 var(...) 引用链取到字面色值 */
function resolveVar(colors: Record<string, string>, key: string, depth = 0): string | null {
  const v = colors[key]
  if (v === undefined || depth > 8) return null
  const m = /^var\((--[a-z0-9-]+)\)$/i.exec(v.trim())
  return m ? resolveVar(colors, m[1], depth + 1) : v
}

/** 主区卡片面的真实像素。半透明时依次压过 content-background 与最坏情况的背景图。
 *
 * 背景图按最亮处（纯白 L=1）取最坏值：landingMoon.png 里的月亮实测就是 L=1.0，
 * 它曾经透到黑灯瞎火的文字底下，同屏底色差 3.2 倍。 */
function realSurface(colors: Record<string, string>): Rgb | null {
  const mainStr = resolveVar(colors, '--color-main-background')
  const main = mainStr ? parseColor(mainStr) : null
  if (!main) return null
  if (main.a >= 1) return main

  const hasImage = (colors['--background-image'] ?? 'none').trim() !== 'none'
  let under: Rgb = hasImage
    ? { r: 255, g: 255, b: 255, a: 1 }
    : { r: 0, g: 0, b: 0, a: 1 }
  const contentStr = resolveVar(colors, '--color-content-background')
  const content = contentStr ? parseColor(contentStr) : null
  if (content) under = over(content, under)
  return over(main, under)
}

interface Measured {
  surface: Rgb
  peak: number
  worst: number
  tiers: { key: string; ratio: number }[]
}

function measure(colors: Record<string, string>, isDark: boolean): Measured | null {
  const surface = realSurface(colors)
  if (!surface) return null
  const keys = isDark ? TIERS_DARK : TIERS_LIGHT
  const tiers: { key: string; ratio: number }[] = []
  for (const key of keys) {
    const raw = resolveVar(colors, key)
    const parsed = raw ? parseColor(raw) : null
    if (!parsed) return null
    tiers.push({ key, ratio: contrast(over(parsed, surface), surface) })
  }
  return {
    surface,
    peak: tiers[0].ratio,
    worst: Math.min(...tiers.map((t) => t.ratio)),
    tiers
  }
}

const failures: string[] = []
let checked = 0

function check(label: string, colors: Record<string, string>, isDark: boolean): void {
  const m = measure(colors, isDark)
  if (!m) {
    failures.push(`${label}：色值解析失败（主区面或字档不是字面色值）`)
    return
  }
  checked += 1
  if (m.worst < AA) {
    const bad = m.tiers.find((t) => t.ratio === m.worst)!
    failures.push(
      `${label}：${bad.key} 仅 ${m.worst.toFixed(2)}:1，低于 AA ${AA}:1` +
        `（面 L=${relativeLuminance(m.surface).toFixed(4)}）`
    )
  }
  if (isDark && m.peak > DARK_PEAK_MAX) {
    failures.push(
      `${label}：峰值 ${m.peak.toFixed(2)}:1 超过深色上限 ${DARK_PEAK_MAX}:1，会光渗刺眼`
    )
  }
}

// ---------------------------------------------------------------- 内置主题
for (const theme of THEMES) {
  for (const level of LEVELS) {
    const colors = applyComfort(buildThemeColors(theme), level, theme.isDark) as Record<
      string,
      string
    >
    check(`【${theme.name}】${level}`, colors, theme.isDark)
  }
}

// ------------------------------------------------------- 自定义主题的默认值
//
// ThemeEditDialog 的 save() 会把表单里所有色值写进配置，所以「默认值」本身就是
// 一套会真实落地的配色，必须一起卡。深色一侧的三个值来自该组件的 MODE_COLORS，
// 两边改动时这里会跟着失败——这正是想要的效果。
const DIALOG_DEFAULTS = {
  light: { font: 'rgb(33, 33, 33)', appBackground: 'rgb(237, 247, 242)', contentBackground: 'rgb(255, 255, 255)' },
  dark: { font: 'rgb(221, 221, 221)', appBackground: 'rgb(0, 0, 0)', contentBackground: 'rgb(37, 37, 39)' }
}

for (const mode of ['light', 'dark'] as const) {
  for (const isDarkFont of [false, true]) {
    const d = DIALOG_DEFAULTS[mode]
    const config = {
      id: 'verify',
      name: '验收',
      isDark: mode === 'dark',
      isDarkFont,
      primary: 'rgb(77, 175, 124)',
      font: d.font,
      appBackground: d.appBackground,
      contentBackground: d.contentBackground,
      sidebarButton: 'rgb(77, 175, 124)',
      badgePrimary: 'rgb(77, 175, 124)',
      badgeSecondary: 'rgb(75, 174, 213)',
      badgeTertiary: 'rgb(231, 170, 54)',
      buttonClose: 'rgb(250, 180, 160)',
      buttonMin: 'rgb(133, 196, 59)',
      buttonHide: 'rgb(59, 194, 178)'
    } as CustomThemeConfig
    const theme = customToThemeDef(config)
    for (const level of LEVELS) {
      const colors = applyComfort(buildThemeColors(theme), level, theme.isDark) as Record<
        string,
        string
      >
      check(`【自定义默认·${mode}·isDarkFont=${isDarkFont}】${level}`, colors, theme.isDark)
    }
  }
}

// -------------------------------------------------- 深色面不得低于 Material 基准
//
// #121212（L≈0.0116）是 Material 的深色基准面。低于它意味着明暗跨度顶到极限，
// 是三个深色主题被投诉的两条共因之一（另一条是峰值过高，已在上面卡住）。
const MATERIAL_DARK_FLOOR = 0.0116
for (const theme of THEMES.filter((t) => t.isDark)) {
  const colors = buildThemeColors(theme) as Record<string, string>
  const surface = realSurface(colors)
  if (!surface) continue
  const lum = relativeLuminance(surface)
  if (lum < MATERIAL_DARK_FLOOR) {
    failures.push(
      `【${theme.name}】主区面 L=${lum.toFixed(4)} 低于 Material 深色基准面 ` +
        `#121212（L=${MATERIAL_DARK_FLOOR}），明暗跨度过大`
    )
  }
}

// -------------------------------------------------------------------- 汇总
if (failures.length) {
  console.error(`主题配色验收未通过（检查 ${checked} 组，${failures.length} 项不达标）：\n`)
  for (const f of failures) console.error(`  ✗ ${f}`)
  console.error('')
  process.exit(1)
}

assert.ok(checked >= THEMES.length * LEVELS.length, '检查组数少于预期，说明有主题被跳过')
console.log(`主题配色验收通过：${checked} 组（${THEMES.length} 内置主题 + 自定义默认值 × ${LEVELS.length} 档位）`)
console.log(`  AA 下限 ${AA}:1 · 深色峰值上限 ${DARK_PEAK_MAX}:1 · 深色面不低于 Material #121212`)
