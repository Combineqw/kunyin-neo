/**
 * 动效包配置。JSON 字段使用 snake_case，运行时字段使用 camelCase。
 * 所有值都直接映射到 CSS custom properties，不依赖第三方动画库。
 */
export type AnimationPackId = 'ios' | 'originos' | 'hyperos4' | 'custom'

export interface AnimationPack {
  id: AnimationPackId
  name: string
  easeStandard: string
  easeSpring: string
  easeSmooth: string
  durationFast: string
  durationBase: string
  durationSlow: string
}

export interface AnimationPackJson {
  name: string
  ease_standard: string
  ease_spring: string
  ease_smooth?: string
  duration_fast?: string
  duration_base: string
  duration_slow?: string
}

export const DEFAULT_ANIMATION_PACKS: readonly AnimationPack[] = [
  {
    id: 'ios',
    name: 'iOS 风格',
    easeStandard: 'cubic-bezier(0.32, 0.72, 0, 1)',
    easeSpring: 'cubic-bezier(0.34, 1.56, 0.64, 1)',
    easeSmooth: 'cubic-bezier(0.22, 0.61, 0.36, 1)',
    durationFast: '0.2s',
    durationBase: '0.35s',
    durationSlow: '0.5s'
  },
  {
    id: 'originos',
    name: 'OriginOS 风格',
    easeStandard: 'cubic-bezier(0.36, 0, 0.66, -0.56)',
    easeSpring: 'cubic-bezier(0.22, 0.9, 0.36, 1)',
    easeSmooth: 'cubic-bezier(0.4, 0, 0.2, 1)',
    durationFast: '0.16s',
    durationBase: '0.28s',
    durationSlow: '0.42s'
  },
  {
    id: 'hyperos4',
    name: '澎湃OS 4 风格',
    easeStandard: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
    easeSpring: 'cubic-bezier(0.22, 1.38, 0.36, 1)',
    easeSmooth: 'cubic-bezier(0.16, 1, 0.3, 1)',
    durationFast: '0.24s',
    durationBase: '0.42s',
    durationSlow: '0.62s'
  }
] as const

export const DEFAULT_CUSTOM_ANIMATION_PACK: AnimationPack = {
  id: 'custom',
  name: '自定义动效',
  easeStandard: DEFAULT_ANIMATION_PACKS[0].easeStandard,
  easeSpring: DEFAULT_ANIMATION_PACKS[0].easeSpring,
  easeSmooth: DEFAULT_ANIMATION_PACKS[0].easeSmooth,
  durationFast: DEFAULT_ANIMATION_PACKS[0].durationFast,
  durationBase: DEFAULT_ANIMATION_PACKS[0].durationBase,
  durationSlow: DEFAULT_ANIMATION_PACKS[0].durationSlow
}

/** 把导入 JSON 规范转换为安全的运行时配置。 */
export function animationPackFromJson(value: unknown, id: AnimationPackId = 'custom'): AnimationPack {
  if (!value || typeof value !== 'object') throw new Error('动效包必须是 JSON 对象')
  const input = value as Partial<AnimationPackJson>
  const required = (key: keyof AnimationPackJson): string => {
    const v = input[key]
    if (typeof v !== 'string' || !v.trim()) throw new Error(`动效包缺少有效字段：${key}`)
    return v.trim()
  }
  const optional = (key: keyof AnimationPackJson, fallback: string): string => {
    const v = input[key]
    return typeof v === 'string' && v.trim() ? v.trim() : fallback
  }
  const base = required('duration_base')
  return {
    id,
    name: required('name'),
    easeStandard: required('ease_standard'),
    easeSpring: required('ease_spring'),
    easeSmooth: optional('ease_smooth', DEFAULT_CUSTOM_ANIMATION_PACK.easeSmooth),
    durationFast: optional('duration_fast', DEFAULT_CUSTOM_ANIMATION_PACK.durationFast),
    durationBase: base,
    durationSlow: optional('duration_slow', DEFAULT_CUSTOM_ANIMATION_PACK.durationSlow)
  }
}

/** 运行时配置导出为稳定的 JSON 字段名。 */
export function animationPackToJson(pack: AnimationPack): AnimationPackJson {
  return {
    name: pack.name,
    ease_standard: pack.easeStandard,
    ease_spring: pack.easeSpring,
    ease_smooth: pack.easeSmooth,
    duration_fast: pack.durationFast,
    duration_base: pack.durationBase,
    duration_slow: pack.durationSlow
  }
}
