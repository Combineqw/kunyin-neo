/**
 * 主题文件（.json）的校验与序列化。
 *
 * 文件内容即 CustomThemeConfig —— 与 settings.appearance.customThemes 的存储结构
 * 完全一致，导入导出零转换。
 *
 * 安全红线（本模块是唯一入口，主进程与渲染层都必须经过这里）：
 * 1. 只接受纯色值（#rgb/#rrggbb/rgb()/rgba()）与白名单内的 CSS 变量引用，
 *    任何 url()/image-set()/expression()/javascript: 一律拒绝；
 * 2. bgImage 只允许本地绝对路径，禁止 http(s)/data/file/blob 等 URL 形态，
 *    因此主题文件无法携带远程背景图；
 * 3. 未知字段一律丢弃（白名单解析而非黑名单过滤），因此无法夹带脚本字段。
 */
import type { CustomThemeConfig, ThemeAuroraConfig } from '../types/settings'

/** 主题文件结构 = 存储结构（CustomThemeConfig），零转换成本。 */
export type ThemeFileConfig = CustomThemeConfig

/** 纯色值：#rgb / #rrggbb / #rrggbbaa / rgb() / rgba()（数值与逗号/空格/斜杠/百分号） */
const COLOR_RE = /^(#[0-9a-f]{3,8}|rgba?\(\s*[\d.\s,%/]+\s*\))$/i
/** 白名单变量引用：只允许引用主题引擎自己派生的 --color-* / --aurora-* 变量 */
const VAR_RE = /^var\(\s*--(?:color|aurora)-[a-z0-9-]+\s*\)$/i
/** 明确禁止的危险构造 */
const FORBIDDEN_RE = /url\(|image-set\(|javascript:|data:|expression\(|<|>|;|@import/i

function isSafeColor(value: string): boolean {
  const v = value.trim()
  if (!v || FORBIDDEN_RE.test(v)) return false
  return COLOR_RE.test(v) || VAR_RE.test(v)
}

/** bgImage：只允许本地绝对路径（Windows 盘符或 POSIX 根），禁止任何 URL scheme。 */
function isSafeLocalPath(value: string): boolean {
  const v = value.trim()
  if (!v) return true // 空串 = 无背景图
  if (/^[a-z][a-z0-9+.-]*:/i.test(v)) return false // http: / file: / data: / blob: ...
  if (FORBIDDEN_RE.test(v)) return false
  return /^([a-z]:[\\/]|[\\/])/i.test(v)
}

const OPTIONAL_COLOR_KEYS = [
  'appBackground',
  'sidebarButton',
  'contentBackground',
  'badgePrimary',
  'badgeSecondary',
  'badgeTertiary',
  'buttonClose',
  'buttonMin',
  'buttonHide'
] as const

/**
 * 解析并校验主题文件内容。失败时抛出带中文原因的 Error，调用方直接展示为 toast。
 * 采用白名单解析：只有下面显式取用的字段会进入结果，其余一概丢弃。
 */
export function themeConfigFromJson(value: unknown): ThemeFileConfig {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('主题文件必须是 JSON 对象')
  }
  const input = value as Record<string, unknown>

  const requiredText = (key: string): string => {
    const v = input[key]
    if (typeof v !== 'string' || !v.trim()) throw new Error(`主题文件缺少有效字段：${key}`)
    return v.trim()
  }
  const requiredColor = (key: string): string => {
    const v = requiredText(key)
    if (!isSafeColor(v)) throw new Error(`字段 ${key} 不是合法的颜色值：${v}`)
    return v
  }

  const id = requiredText('id')
  if (!/^[a-zA-Z0-9._-]+$/.test(id)) {
    throw new Error('主题 id 只能包含字母、数字、点、下划线和连字符')
  }
  const name = requiredText('name')
  if (name.length > 40) throw new Error('主题名称过长（上限 40 字）')
  // 名称只作为文本渲染（Vue 插值会转义，非 v-html），不构成注入面；
  // 但尖括号/控制字符不属于正常主题名，一并拒绝，避免出现伪装成标记的名字
  // eslint-disable-next-line no-control-regex
  if (/[<>]|[\u0000-\u001f\u007f]/.test(name)) {
    throw new Error('主题名称包含非法字符')
  }

  const bgImage = typeof input.bgImage === 'string' ? input.bgImage.trim() : ''
  if (!isSafeLocalPath(bgImage)) {
    throw new Error('背景图只允许本地文件路径，不接受网络地址')
  }

  const config: ThemeFileConfig = {
    id,
    name,
    isDark: input.isDark === true,
    isDarkFont: input.isDarkFont === true,
    primary: requiredColor('primary'),
    font: typeof input.font === 'string' && input.font.trim() ? requiredColor('font') : '',
    bgImage
  }

  for (const key of OPTIONAL_COLOR_KEYS) {
    const v = input[key]
    if (typeof v !== 'string' || !v.trim()) continue
    if (!isSafeColor(v)) throw new Error(`字段 ${key} 不是合法的颜色值：${v}`)
    config[key] = v.trim()
  }

  const aurora = input.aurora
  if (aurora && typeof aurora === 'object' && !Array.isArray(aurora)) {
    const src = aurora as Record<string, unknown>
    const out: ThemeAuroraConfig = {}
    for (const key of ['c1', 'c3', 'glow'] as const) {
      const v = src[key]
      if (typeof v !== 'string' || !v.trim()) continue
      if (!isSafeColor(v)) throw new Error(`极光颜色 aurora.${key} 不是合法的颜色值：${v}`)
      out[key] = v.trim()
    }
    if (Object.keys(out).length) config.aurora = out
  }

  return config
}

/** 序列化为主题文件内容（字段顺序稳定，便于人工编辑与 diff）。 */
export function themeConfigToJson(config: ThemeFileConfig): ThemeFileConfig {
  // 先过一遍校验，保证导出的文件一定能被导入（也防止把非法值写进文件）
  return themeConfigFromJson({ ...config })
}

/** 文件名安全化（与 animation 导出保持一致的处理）。 */
export function themeFileName(name: string): string {
  const safe = name.replace(/[\\/:*?"<>|]/g, '_').trim() || 'theme'
  return `${safe}.kytheme.json`
}
