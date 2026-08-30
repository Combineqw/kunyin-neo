/**
 * 字体工具：枚举系统字体 + 应用软件字体到 :root。
 *
 * 枚举优先用 Local Font Access API（Electron 39 支持 window.queryLocalFonts），
 * 失败则回退到按 OS 的常见字体清单，保证下拉总有可选项。
 */

/** 各 OS 常见字体兜底清单（枚举失败时用） */
const FALLBACK_FONTS: Record<string, string[]> = {
  win32: [
    'Microsoft YaHei',
    'Microsoft YaHei UI',
    'SimSun',
    'SimHei',
    'KaiTi',
    'FangSong',
    'Segoe UI',
    'Arial',
    'Tahoma',
    'Consolas'
  ],
  darwin: [
    'PingFang SC',
    'Hiragino Sans GB',
    'STHeiti',
    'STSong',
    'Helvetica Neue',
    'Arial',
    'Menlo'
  ],
  linux: [
    'Source Han Sans SC',
    'Noto Sans CJK SC',
    'WenQuanYi Micro Hei',
    'Ubuntu',
    'DejaVu Sans',
    'Droid Sans'
  ]
}

function normalizeFamilyName(value: unknown): string {
  // 控制字符正是要剔除的目标。字体名来自系统枚举，夹带控制字符会破坏后面拼出的
  // CSS 声明；下方 isSafeFamilyName 再挡引号与反斜杠，两道一起构成输入清洗。
  // eslint-disable-next-line no-control-regex
  return typeof value === 'string' ? value.replace(/[\u0000-\u001f\u007f]/g, '').trim() : ''
}

/** 仅允许作为 CSS 字体族名使用的普通系统字体名称，避免引号/反斜杠破坏声明。 */
function isSafeFamilyName(value: string): boolean {
  return !!value && !/["\\]/.test(value)
}

function isAvailableFamilyName(value: string): boolean {
  if (!isSafeFamilyName(value)) return false
  try {
    return document.fonts.check(`12px "${value}"`)
  } catch {
    return true
  }
}

/** 枚举系统已安装字体族名（去重排序）。失败回退常见清单。 */
export async function listSystemFonts(platform: string): Promise<string[]> {
  const q = (window as unknown as { queryLocalFonts?: () => Promise<{ family: string }[]> })
    .queryLocalFonts
  if (typeof q === 'function') {
    try {
      const fonts = await q()
      const families = Array.from(
        new Set(fonts.map((f) => normalizeFamilyName(f.family)).filter(isSafeFamilyName))
      )
      if (families.length) return families.sort((a, b) => a.localeCompare(b))
    } catch {
      /* 权限被拒或不支持，走兜底 */
    }
  }
  return (FALLBACK_FONTS[platform] ?? FALLBACK_FONTS.win32).filter(isSafeFamilyName)
}

/**
 * 应用软件界面字体：只把经过清洗的字体族名写入 :root。
 * 空值或异常名称会移除自定义字体，让 base.css 使用稳定的系统回退栈。
 */
export function applyAppFont(font: string): void {
  const root = document.documentElement
  const family = normalizeFamilyName(font)
  if (!isAvailableFamilyName(family)) {
    root.style.removeProperty('--app-font')
    return
  }
  root.style.setProperty('--app-font', `"${family}"`)
}
