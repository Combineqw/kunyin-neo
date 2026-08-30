// 主题定义与色阶生成。移植自 lx-music-desktop（Apache-2.0, © lyswhut）的
// src/common/theme/{index.json, utils.js, createThemes.js}，改写为 TS。
// 色阶（--color-primary-dark-*/light-*、字色 --color-000~1000）由 createThemeColors
// 从主色/字色派生，与 LX 的 index.json 展开结果一致；ext 对应其 extInfo 语义覆盖。
import { RGB_Alpha_Shade, RGB_Linear_Shade } from './colorUtils'
import type { CustomThemeConfig, ThemeFileConfig } from '@common'
import bgLandingMoon from '../assets/theme/landingMoon.png'
import bgJqbg from '../assets/theme/jqbg.jpg'
import bgMyzc from '../assets/theme/myzcbg.jpg'
import bgChinaInk from '../assets/theme/china_ink.jpg'
import bgXnkl from '../assets/theme/xnkl.png'

export type { CustomThemeConfig }

export type ThemeColors = Record<string, string>

export interface ThemeDef {
  id: string
  name: string
  isDark: boolean
  isDarkFont: boolean
  /** 是否用户自定义主题 */
  isCustom?: boolean
  /** 品牌主色 rgb(...) */
  primary: string
  /** 正文字色 rgb(...) */
  font: string
  /** 语义变量覆盖（--color-app-background / --background-image 等） */
  ext: ThemeColors
}

/** 用户自定义主题配置（存于 settings.appearance.customThemes，跨进程传输的纯数据） */

/** 由 primary/font 派生完整 --color-primary-* 与字色梯度 */
export function createThemeColors(
  rgbaColor: string,
  fontRgbaColor: string,
  isDark: boolean,
  isDarkFont: boolean
): ThemeColors {
  const colors: ThemeColors = { '--color-primary': rgbaColor }

  let preColor = rgbaColor
  for (let i = 1; i < 11; i += 1) {
    preColor = RGB_Linear_Shade(isDark ? 0.2 : -0.1, preColor)
    colors[`--color-primary-dark-${i * 100}`] = preColor
    for (let j = 1; j < 10; j += 1) {
      colors[`--color-primary-dark-${i * 100}-alpha-${j * 100}`] = RGB_Alpha_Shade(
        0.1 * j,
        preColor
      )
      colors[`--color-primary-alpha-${j * 100}`] = RGB_Alpha_Shade(0.1 * j, rgbaColor)
    }
  }
  preColor = rgbaColor
  for (let i = 1; i < 10; i += 1) {
    preColor = RGB_Linear_Shade(isDark ? -0.1 : 0.2, preColor)
    colors[`--color-primary-light-${i * 100}`] = preColor
    for (let j = 1; j < 10; j += 1) {
      colors[`--color-primary-light-${i * 100}-alpha-${j * 100}`] = RGB_Alpha_Shade(
        0.1 * j,
        preColor
      )
    }
  }
  preColor = RGB_Linear_Shade(isDark ? -0.35 : 1, preColor)
  colors['--color-primary-light-1000'] = preColor
  for (let j = 1; j < 10; j += 1) {
    colors[`--color-primary-light-1000-alpha-${j * 100}`] = RGB_Alpha_Shade(0.1 * j, preColor)
  }

  colors['--color-theme'] = isDark ? colors['--color-primary-light-900'] : rgbaColor

  return { ...colors, ...createFontColors(fontRgbaColor, isDark, isDarkFont) }
}

function createFontColors(rgbaColor: string, isDark: boolean, isDarkFont: boolean): ThemeColors {
  rgbaColor ||= isDark ? 'rgb(229, 229, 229)' : 'rgb(33, 33, 33)'
  if (isDark) {
    const colors: ThemeColors = { '--color-1000': rgbaColor }
    const step = isDarkFont ? -0.015 : -0.05
    let preColor = rgbaColor
    for (let i = 1; i < 21; i += 1) {
      preColor = RGB_Linear_Shade(step, preColor)
      colors[`--color-${String(1000 - 50 * i).padStart(3, '0')}`] = preColor
    }
    return colors
  }
  const colors: ThemeColors = { '--color-1000': rgbaColor }
  const step = isDarkFont ? 0.02 : 0.05
  for (let i = 1; i < 21; i += 1) {
    colors[`--color-${String(1000 - 50 * i).padStart(3, '0')}`] = RGB_Linear_Shade(
      step * i,
      rgbaColor
    )
  }
  return colors
}

// ---- extInfo 模板（对应 LX index.json 各主题的 extInfo） ----

/** 无背景图浅色主题共用模板
 *
 * 卡片底原为纯白 rgba(255,255,255,1)（相对亮度 1.0）。设置页这类大面积卡片铺满时
 * 实测刺眼，故降到 rgb(250,248,245)（L=0.9405，降 5.9%，略偏暖）。正文 --color-850
 * 对它仍有 9.48:1。带背景图的浅色主题卡片底是半透明的（0.7~0.8），本就不刺眼，不改。
 *
 * --color-font-label 原取 --color-450，对纯白仅 2.78:1，连大字 3:1 都不到——
 * 亮却看不清的根源。改取 --color-650（对新底色 4.74:1，过 AA），
 * 是过 AA 的最浅一档，与正文 9.48:1 仍有明显层级差。 */
const lightExt = (badgeSecondary: string, badgeTertiary: string): ThemeColors => ({
  '--color-app-background': 'var(--color-primary-light-600-alpha-700)',
  '--color-main-background': 'rgb(250, 248, 245)',
  '--color-nav-font': 'var(--color-primary)',
  '--color-font-label': 'var(--color-650)',
  '--background-image': 'none',
  '--color-badge-primary': 'var(--color-primary)',
  '--color-badge-secondary': badgeSecondary,
  '--color-badge-tertiary': badgeTertiary
})

/* ---- 深色主题眩光：三个深色主题共有的问题 ----

   用户先反馈极夜/深海伤眼，我改完后又指出黑灯瞎火同样伤眼——说明这不是个别主题
   配色跑偏，而是三者共有的结构问题。实测（卡片面为背景合成后的真实像素）：

     主题        面 L      峰值(1000档)   正文(850)   字色饱和
     黑灯瞎火    0.0074    14.51:1        10.59:1     0%
     极夜        0.0074    13.94:1        12.68:1     64%（改前）
     深海        0.0074    13.56:1        12.37:1     50%（改前）

   两条共性：
     1) 面太黑。0.0074 远低于 Material 深色基准面 #121212（L≈0.0116），
        明暗跨度顶到极限。
     2) 顶太亮。峰值 13.5~14.5:1 超出深色模式的舒适上限（一般建议 ≤12:1），
        即使饱和度为 0 也会光渗（halation）——所以我第一轮拿黑灯瞎火当
        「0% 饱和 = 不刺眼」的参照组是错的，参照组本身就超标。

   另有一条只属于黑灯瞎火：它的主区面原本是 0.9 半透明，背景图那块 L=1.0 的月亮
   会透到文字底下，同屏底色亮度差 3.2 倍（0.0074 vs 0.0238），眼睛要反复重新适应。

   改法：
     - 三者主区面一律抬到不透明、L≈0.019~0.021，峰值压到 11.2:1 附近。
       不透明同时消掉月亮透字的问题；侧栏仍用全透明的 app-background，
       月亮从那条窄带露出，主题辨识度不丢。
     - 极夜/深海的字色额外从高饱和青色降到近中性冷灰（22% / 16%），
       彩色字叠近黑底是它们额外多出来的一层光渗。颜色改由背景色相与
       极光光带承担——光带是小面积装饰，保持原本高饱和不动。
   在用的四个字档（1000/850/750/550，深色下 font-label 被覆盖到 550）全部过 AA，
   最差 4.55:1；黑灯瞎火的正文↔说明灰阶差保持 50，层级没被压平。 */

/** 深色主区面的统一基准。见上方「深色主题眩光」注释：0.9 半透明压在近黑的
 * content-background 上，真实像素只有 L≈0.0056，比 Material 深色基准面
 * #121212（L≈0.0116）还暗一半，峰值对比顶到 14.98:1。改为不透明后 L=0.0186，
 * 且不再随底下的背景图变化——同屏底色不会一处 0.0056 一处 0.0238。 */
const DARK_SURFACE = 'rgb(37, 37, 39)'

/** 无背景图深色主题共用模板（等价于 black 主题的 extInfo，抽出来复用） */
const darkExt = (badgeSecondary: string, badgeTertiary: string): ThemeColors => ({
  '--color-app-background': 'rgba(0, 0, 0, 0)',
  '--color-main-background': DARK_SURFACE,
  '--color-nav-font': 'var(--color-primary)',
  '--background-image': 'none',
  '--color-badge-primary': 'var(--color-primary)',
  '--color-badge-secondary': badgeSecondary,
  '--color-badge-tertiary': badgeTertiary
})

/** 带背景图浅色主题模板（app/main 背景半透明，透出 #app 层的背景图）
 * 卡片底半透明（0.7~0.8）不刺眼，故保留原值；但说明文字同样从 --color-450
 * 提到 --color-650，理由见 lightExt。 */
const lightBgExt = (
  image: string,
  appBg: string,
  mainBg: string,
  badgeSecondary: string,
  badgeTertiary: string,
  extra?: ThemeColors
): ThemeColors => ({
  '--color-app-background': appBg,
  '--color-main-background': mainBg,
  '--color-nav-font': 'var(--color-primary)',
  '--color-font-label': 'var(--color-650)',
  '--background-image': `url(${image})`,
  '--color-badge-primary': 'var(--color-primary)',
  '--color-badge-secondary': badgeSecondary,
  '--color-badge-tertiary': badgeTertiary,
  ...extra
})

/** 内置主题（完整移植自 lx-music-desktop 的 15 个默认主题） */
export const THEMES: ThemeDef[] = [
  {
    id: 'green',
    name: '绿意盎然',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(77, 175, 124)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#4baed5', '#e7aa36')
  },
  {
    id: 'blue',
    name: '蓝田生玉',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(52, 152, 219)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#5cbf9b', '#5cbf9b')
  },
  {
    id: 'blue_plus',
    name: '蛋雅深蓝',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(77, 131, 175)',
    font: 'rgb(33, 33, 33)',
    ext: {
      ...lightExt('rgba(66.6, 150.7, 171, 1)', 'rgba(54, 196, 231, 1)'),
      '--color-app-background': 'var(--color-primary-light-600-alpha-600)'
    }
  },
  {
    id: 'orange',
    name: '橙黄橘绿',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(245, 171, 53)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#9ed458', '#9ed458')
  },
  {
    id: 'red',
    name: '热情似火',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(214, 69, 65)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#dfbb6b', '#dfbb6b')
  },
  {
    id: 'pink',
    name: '粉装玉琢',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(241, 130, 141)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#f5b684', '#f5b684')
  },
  {
    id: 'purple',
    name: '重斤球紫',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(155, 89, 182)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#e5a39f', '#e5a39f')
  },
  {
    id: 'grey',
    name: '灰常美丽',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(108, 122, 137)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#b19b9f', '#b19b9f')
  },
  {
    id: 'ming',
    name: '青出于黑',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(51, 110, 123)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#6376a2', '#6376a2')
  },
  {
    id: 'blue2',
    name: '清热板蓝',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(79, 98, 208)',
    font: 'rgb(33, 33, 33)',
    ext: lightExt('#b080db', '#b080db')
  },
  {
    id: 'black',
    name: '黑灯瞎火',
    isDark: true,
    isDarkFont: false,
    primary: 'rgb(150, 150, 150)',
    // 见下方「深色主题眩光」注释。字色 229 → 221，主区面抬到不透明 rgb(37,37,39)：
    // 峰值 14.51:1 → 11.26:1，且月亮不再透到文字底下（原来同屏底色差 3.2 倍）。
    font: 'rgb(221, 221, 221)',
    ext: {
      '--color-app-background': 'rgba(0, 0, 0, 0)',
      // 不透明：侧栏仍是全透明的 app-background，月亮从那条窄带露出，主题辨识度保留。
      '--color-main-background': DARK_SURFACE,
      '--color-nav-font': 'var(--color-primary)',
      '--background-image': `url(${bgLandingMoon})`,
      '--color-badge-primary': 'var(--color-primary-dark-200)',
      '--color-badge-secondary': 'var(--color-primary)',
      '--color-badge-tertiary': 'var(--color-primary-dark-300)'
    }
  },
  {
    id: 'mid_autumn',
    name: '月里嫦娥',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(74, 55, 82)',
    font: 'rgb(33, 33, 33)',
    ext: lightBgExt(
      bgJqbg,
      'rgba(255, 255, 255, 0)',
      'rgba(255, 255, 255, 0.76)',
      '#af9479',
      '#af9479',
      { '--color-nav-font': 'var(--color-primary-light-600)' }
    )
  },
  {
    id: 'naruto',
    name: '木叶之村',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(87, 144, 167)',
    font: 'rgb(33, 33, 33)',
    ext: lightBgExt(
      bgMyzc,
      'rgba(255, 255, 255, 0.08)',
      'rgba(255, 255, 255, 0.7)',
      'var(--color-primary-light-100)',
      'var(--color-primary-light-100)'
    )
  },
  {
    id: 'china_ink',
    name: '近墨者黑',
    isDark: false,
    isDarkFont: false,
    primary: 'rgba(47, 47, 47, 1)',
    font: 'rgb(33, 33, 33)',
    ext: lightBgExt(
      bgChinaInk,
      'rgba(255, 255, 255, 0)',
      'rgba(255, 255, 255, 0.8)',
      'rgba(67, 139, 65, 1)',
      'rgba(132, 135, 65, 1)',
      {
        '--color-badge-primary': 'rgba(137, 70, 70, 1)',
        '--color-btn-hide': 'rgba(183, 212, 208, 1)',
        '--color-btn-min': 'rgba(200, 214, 183, 1)',
        '--color-btn-close': 'rgba(218, 195, 188, 1)'
      }
    )
  },
  {
    id: 'happy_new_year',
    name: '新年快乐',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(192, 57, 43)',
    font: 'rgb(33, 33, 33)',
    ext: lightBgExt(
      bgXnkl,
      'rgba(255, 255, 255, 0.15)',
      'rgba(255, 255, 255, 0.8)',
      '#dfbb6b',
      'var(--color-primary-light-100)',
      { '--color-badge-primary': '#7fb575' }
    )
  },

  // ---------------- 极光系（.aurora-band 光带随主题变色） ----------------
  // 这五个主题的主色刻意取极淡/高亮色：作为背景与光带很好看，但直接当字色对比度
  // 只有 1.19~1.31，远不足 AA。因此浅色三主题把"字色语义变量"下移到
  // --color-primary-dark-700（实测白底 4.95/5.09/5.29、侧栏 4.80/4.90/5.03，全部过 AA），
  // 而 --color-primary 本身保持需求给定的原值不动，背景/光带观感不受影响。
  {
    id: 'aurora_mermaid',
    name: '人鱼姬',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(242, 231, 253)',
    font: 'rgb(33, 33, 33)',
    ext: {
      ...lightExt('#4bb3d5', '#c9a0e8'),
      '--color-nav-font': 'var(--color-primary-dark-700)',
      '--color-primary-font': 'var(--color-primary-dark-700)',
      '--color-button-font': 'var(--color-primary-dark-800)',
      '--color-badge-primary': 'var(--color-primary-dark-700)',
      '--aurora-c1': '#f2e7fd',
      '--aurora-c3': '#e3f6fd'
    }
  },
  {
    id: 'aurora_polar_night',
    name: '极夜',
    isDark: true,
    isDarkFont: true,
    // 深色 + isDarkFont 的字色梯度衰减极缓（step -0.015），在用四档最差 8.56:1
    primary: 'rgb(122, 190, 214)',
    font: 'rgb(214, 224, 232)',
    ext: {
      ...darkExt('#8f8fd0', '#7abed6'),
      // 主区面抬到不透明 rgb(34,39,47)（L=0.0200）：第一轮改到 0.0110 仍低于
      // Material 深色基准面 #121212（L≈0.0116），峰值 12.87:1 也还偏高。
      '--color-main-background': 'rgb(34, 39, 47)',
      // 光带是小面积装饰，保持原本的高饱和极光色，主题辨识度靠它而不是靠字色。
      '--aurora-c1': '#8fe0f5',
      '--aurora-c3': '#b9a8ff',
      '--aurora-glow': 'rgba(143, 224, 245, 0.4)'
    }
  },
  {
    id: 'aurora_morning_mist',
    name: '晨雾',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(245, 227, 200)',
    font: 'rgb(33, 33, 33)',
    ext: {
      ...lightExt('#c99a6b', '#a8b58c'),
      '--color-nav-font': 'var(--color-primary-dark-700)',
      '--color-primary-font': 'var(--color-primary-dark-700)',
      '--color-button-font': 'var(--color-primary-dark-800)',
      '--color-badge-primary': 'var(--color-primary-dark-700)',
      '--aurora-c1': '#f5e3c8',
      '--aurora-c3': '#f0ddd0'
    }
  },
  {
    id: 'aurora_sunset_glow',
    name: '霞光',
    isDark: false,
    isDarkFont: false,
    primary: 'rgb(255, 217, 200)',
    font: 'rgb(33, 33, 33)',
    ext: {
      ...lightExt('#e08a7a', '#c78ac9'),
      '--color-nav-font': 'var(--color-primary-dark-700)',
      '--color-primary-font': 'var(--color-primary-dark-700)',
      '--color-button-font': 'var(--color-primary-dark-800)',
      '--color-badge-primary': 'var(--color-primary-dark-700)',
      '--aurora-c1': '#ffd9c8',
      '--aurora-c3': '#f0d0f5'
    }
  },
  {
    id: 'aurora_deep_sea',
    name: '深海',
    isDark: true,
    isDarkFont: true,
    primary: 'rgb(110, 185, 182)',
    font: 'rgb(212, 226, 226)',
    ext: {
      ...darkExt('#7f9fd0', '#6eb9b6'),
      '--color-main-background': 'rgb(31, 41, 45)',
      '--aurora-c1': '#7fe0d8',
      '--aurora-c3': '#a8c8ff',
      '--aurora-glow': 'rgba(127, 224, 216, 0.4)'
    }
  }
]

/** 组装某主题的完整 CSS 变量（派生梯度 + 语义覆盖） */
export function buildThemeColors(theme: ThemeDef): ThemeColors {
  return {
    ...createThemeColors(theme.primary, theme.font, theme.isDark, theme.isDarkFont),
    ...theme.ext
  }
}

/** 自定义主题配置 → 主题定义（背景图绝对路径转 file:/// URL，其余走浅色/深色默认模板） */
export function customToThemeDef(config: CustomThemeConfig): ThemeDef {
  const bgImage = config.bgImage
    ? `url(file:///${encodeURI(config.bgImage.replaceAll('\\', '/'))})`
    : 'none'
  const ext: ThemeColors = config.isDark
    ? {
        '--color-app-background': 'rgba(0, 0, 0, 0)',
        // 与内置深色主题同一基准面，理由见 DARK_SURFACE。用户若想让背景图透到
        // 主区，可在编辑框里自行填 contentBackground 覆盖（下面的 if 会生效）。
        '--color-main-background': DARK_SURFACE,
        '--color-nav-font': 'var(--color-primary)',
        '--background-image': bgImage,
        '--color-badge-primary': 'var(--color-primary)',
        '--color-badge-secondary': '#4baed5',
        '--color-badge-tertiary': '#e7aa36'
      }
    : {
        ...lightExt('#4baed5', '#e7aa36'),
        // 有背景图时主区半透明透出背景
        ...(config.bgImage
          ? {
              '--color-app-background': 'rgba(255, 255, 255, 0.15)',
              '--color-main-background': 'rgba(255, 255, 255, 0.85)'
            }
          : {}),
        '--background-image': bgImage
      }
  if (config.appBackground) ext['--color-app-background'] = config.appBackground
  if (config.contentBackground) ext['--color-main-background'] = config.contentBackground
  if (config.sidebarButton) ext['--color-nav-font'] = config.sidebarButton
  if (config.badgePrimary) ext['--color-badge-primary'] = config.badgePrimary
  if (config.badgeSecondary) ext['--color-badge-secondary'] = config.badgeSecondary
  if (config.badgeTertiary) ext['--color-badge-tertiary'] = config.badgeTertiary
  if (config.buttonClose) ext['--color-btn-close'] = config.buttonClose
  if (config.buttonMin) ext['--color-btn-min'] = config.buttonMin
  if (config.buttonHide) ext['--color-btn-hide'] = config.buttonHide
  // 极光光带覆写（导入的主题文件可携带；值已由 themeConfigFromJson 校验为纯色）
  const { aurora } = config
  if (aurora?.c1) ext['--aurora-c1'] = aurora.c1
  if (aurora?.c3) ext['--aurora-c3'] = aurora.c3
  if (aurora?.glow) ext['--aurora-glow'] = aurora.glow
  return {
    id: config.id,
    name: config.name,
    isDark: config.isDark,
    isDarkFont: config.isDarkFont ?? false,
    isCustom: true,
    primary: config.primary,
    font: config.font,
    ext
  }
}

export function findTheme(id: string, customs: ThemeDef[] = []): ThemeDef | undefined {
  return THEMES.find((t) => t.id === id) ?? customs.find((t) => t.id === id)
}

/** ext 里的值可能是 var(...) 引用；导出文件只保留能独立成立的具体色值。 */
function concreteColor(value: string | undefined): string | undefined {
  if (!value) return undefined
  const v = value.trim()
  if (!v || v.startsWith('var(')) return undefined
  return /^(#|rgba?\()/i.test(v) ? v : undefined
}

/**
 * 主题定义 → 可导出的主题文件配置。内置主题同样支持（导出后改个色再导入即成为
 * 自定义主题）。背景图是打包后的构建产物 URL，无法在别的机器上还原为文件路径，
 * 故导出时置空；导入方会落到对应的浅色/深色默认模板。
 */
export function themeDefToFileConfig(theme: ThemeDef): ThemeFileConfig {
  const config: ThemeFileConfig = {
    id: theme.id,
    name: theme.name,
    isDark: theme.isDark,
    isDarkFont: theme.isDarkFont,
    primary: theme.primary,
    font: theme.font,
    bgImage: ''
  }
  const map: [keyof ThemeFileConfig, string][] = [
    ['appBackground', '--color-app-background'],
    ['contentBackground', '--color-main-background'],
    ['sidebarButton', '--color-nav-font'],
    ['badgePrimary', '--color-badge-primary'],
    ['badgeSecondary', '--color-badge-secondary'],
    ['badgeTertiary', '--color-badge-tertiary'],
    ['buttonClose', '--color-btn-close'],
    ['buttonMin', '--color-btn-min'],
    ['buttonHide', '--color-btn-hide']
  ]
  for (const [key, cssVar] of map) {
    const v = concreteColor(theme.ext[cssVar])
    if (v) (config[key] as string) = v
  }
  const c1 = concreteColor(theme.ext['--aurora-c1'])
  const c3 = concreteColor(theme.ext['--aurora-c3'])
  const glow = concreteColor(theme.ext['--aurora-glow'])
  if (c1 || c3 || glow) {
    config.aurora = { ...(c1 && { c1 }), ...(c3 && { c3 }), ...(glow && { glow }) }
  }
  return config
}

/** 导入去重：id 冲突时自动加 -1/-2 后缀；名称同步标注副本序号。 */
export function dedupeThemeIdentity(
  config: ThemeFileConfig,
  existing: { id: string; name: string }[]
): ThemeFileConfig {
  const taken = new Set([...THEMES.map((t) => t.id), ...existing.map((t) => t.id)])
  if (!taken.has(config.id)) return config
  let n = 1
  while (taken.has(`${config.id}-${n}`)) n += 1
  const names = new Set(existing.map((t) => t.name))
  let name = `${config.name} (${n})`
  let m = n
  while (names.has(name)) {
    m += 1
    name = `${config.name} (${m})`
  }
  return { ...config, id: `${config.id}-${n}`, name }
}
