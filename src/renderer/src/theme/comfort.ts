/**
 * 界面亮度档位（护眼）。
 *
 * 做法：在主题引擎出口对算好的色值做一次后处理。浅色压「大面积亮面」不碰字色，
 * 深色反过来压「最亮的字档」不碰底色——两种模式的眩光成因不同，见 applyComfort。
 * 好处是 19 个内置主题 + 用户自定义主题全部自动生效，不用逐个维护；
 * 档位设为「标准」时函数原样返回，等于完全关闭，恢复主题本来的样子。
 *
 * 只处理字面 rgb()/rgba()/#hex 值。var(...) 引用不动——语义变量（如
 * --color-content-background: var(--color-primary-light-1000)）会随它引用的
 * 梯度色一起变，改梯度就够了，重复处理反而会叠加两次。
 */

import type { ComfortLevel } from '@common'

export type { ComfortLevel }

export const COMFORT_LEVELS: readonly { id: ComfortLevel; name: string; desc: string }[] = [
  { id: 'standard', name: '标准', desc: '主题原本的亮度' },
  { id: 'soft', name: '柔和', desc: '浅色亮面降约 10%；深色峰值对比降到 10:1' },
  { id: 'softer', name: '极柔和', desc: '浅色亮面降约 19%；深色峰值对比降到 9:1' }
] as const

/** 各档位的压暗强度 */
const STRENGTH: Record<ComfortLevel, number> = {
  standard: 0,
  soft: 0.045,
  softer: 0.09
}

/** 只处理相对亮度高于此值的色（大面积亮面）。中间调与深色不动，避免整体发灰。 */
const LUM_THRESHOLD = 0.7

/** 暖偏移：蓝通道多压一点、红通道少压一点，压暗的同时略微偏暖，比纯灰舒服。 */
const WARM = 0.35

const RGB_RE = /^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*(?:,\s*([\d.]+)\s*)?\)$/i
const HEX_RE = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i

type Rgba = { r: number; g: number; b: number; a: number | null }

function parseColor(value: string): Rgba | null {
  const v = value.trim()
  const m = RGB_RE.exec(v)
  if (m) {
    return {
      r: Number(m[1]),
      g: Number(m[2]),
      b: Number(m[3]),
      a: m[4] === undefined ? null : Number(m[4])
    }
  }
  const h = HEX_RE.exec(v)
  if (!h) return null
  let hex = h[1]
  if (hex.length === 3) {
    hex = hex
      .split('')
      .map((c) => c + c)
      .join('')
  }
  return {
    r: parseInt(hex.slice(0, 2), 16),
    g: parseInt(hex.slice(2, 4), 16),
    b: parseInt(hex.slice(4, 6), 16),
    a: null
  }
}

/** WCAG 相对亮度 */
function relativeLuminance({ r, g, b }: Rgba): number {
  const ch = (c: number): number => {
    const s = c / 255
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
  }
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}

function formatColor({ r, g, b, a }: Rgba): string {
  const clamp = (c: number): number => Math.max(0, Math.min(255, Math.round(c)))
  return a === null
    ? `rgb(${clamp(r)}, ${clamp(g)}, ${clamp(b)})`
    : `rgba(${clamp(r)}, ${clamp(g)}, ${clamp(b)}, ${a})`
}

function dim(color: Rgba, k: number): Rgba {
  return {
    r: color.r * (1 - k * (1 - WARM * 0.5)),
    g: color.g * (1 - k),
    b: color.b * (1 - k * (1 + WARM)),
    a: color.a
  }
}

/** 浅色下字色要跟着底一起压暗，否则对比度会掉。
 *
 * 压底之后底变暗、字不变 → 对比度下降。实测「极柔和」档说明文字（--color-650）
 * 会从 4.74:1 掉到 3.89:1，跌破 AA。字色按同一因子压不够（4.45:1 仍不达标），
 * 数值求解得到需要约 1.33 倍于底的压强，才能让对比度不低于原始值。 */
const LIGHT_FONT_FACTOR = 1.33

/** 深色档位的压暗强度。比浅色大一倍：浅色压的是大面积底，深色压的是小面积字，
 * 同样的强度在字上几乎看不出变化。 */
const DARK_STRENGTH: Record<ComfortLevel, number> = {
  standard: 0,
  soft: 0.09,
  softer: 0.18
}

/** 深色下只压亮度高于此值的字档。低档位（说明文字那一带）本就贴近 AA 边界，
 * 再压就不达标了；而眩光来自最亮的那几档，压它们才有用。 */
const DARK_FONT_FLOOR = 0.35

/** 深色：按亮度加权压暗一个字色。越亮压得越多，低于下限完全不动。 */
function dimFontForDark(color: Rgba, k: number): Rgba {
  const l = relativeLuminance(color)
  if (l < DARK_FONT_FLOOR) return color
  const w = (l - DARK_FONT_FLOOR) / (1 - DARK_FONT_FLOOR)
  const f = 1 - k * w
  return { r: color.r * f, g: color.g * f, b: color.b * f, a: color.a }
}

/**
 * 按档位调整一组主题变量，降低长时间注视的负担。
 *
 * 浅色与深色的着力点相反，所以走两条路径：
 *   - 浅色：大面积是亮面，压底色；字色按 LIGHT_FONT_FACTOR 一并压暗以守住对比度。
 *   - 深色：大面积是暗面，底色已在主题定义里抬到 L≈0.02，这里压的是「顶」——
 *     最亮的那几个字档。深色眩光的成因是峰值对比过高（实测原始三个深色主题
 *     13.5~14.5:1，超出深色模式建议的 12:1 上限），压峰值才对症。
 *
 * @param colors 主题引擎算好的完整变量表
 * @param level  亮度档位
 * @param isDark 当前主题是否深色
 */
export function applyComfort<T extends Record<string, string>>(
  colors: T,
  level: ComfortLevel,
  isDark: boolean
): T {
  const k = isDark ? (DARK_STRENGTH[level] ?? 0) : (STRENGTH[level] ?? 0)
  if (k <= 0) return colors

  const out: Record<string, string> = { ...colors }
  for (const [key, value] of Object.entries(colors)) {
    if (typeof value !== 'string') continue
    const isGradient = /^--color-\d{3,4}$/.test(key)

    if (isDark) {
      // 只处理字色梯度：语义字色多是 var(...) 引用，会随梯度一起变，重复处理会叠两次。
      if (!isGradient) continue
      const parsed = parseColor(value)
      if (!parsed) continue
      out[key] = formatColor(dimFontForDark(parsed, k))
      continue
    }

    // 浅色的字色梯度跟着底一起压暗（见 LIGHT_FONT_FACTOR）。语义字色多是 var(...)
    // 引用，会随梯度一起变，不单独处理，否则叠两次。
    if (isGradient) {
      const pf = parseColor(value)
      if (!pf) continue
      const f = 1 - k * LIGHT_FONT_FACTOR
      out[key] = formatColor({ r: pf.r * f, g: pf.g * f, b: pf.b * f, a: pf.a })
      continue
    }
    if (key.includes('font')) continue

    const parsed = parseColor(value)
    if (!parsed) continue
    if (relativeLuminance(parsed) < LUM_THRESHOLD) continue
    out[key] = formatColor(dim(parsed, k))
  }
  return out as T
}
