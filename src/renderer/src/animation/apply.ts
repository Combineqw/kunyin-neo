import {
  DEFAULT_CUSTOM_ANIMATION_PACK,
  DEFAULT_ANIMATION_PACKS,
  type AnimationPack,
  type AnimationPackId
} from '@common'

function byId(id: AnimationPackId): AnimationPack | undefined {
  return DEFAULT_ANIMATION_PACKS.find((pack) => pack.id === id)
}

export function resolveAnimationPack(
  id: AnimationPackId | string,
  customPack: AnimationPack
): AnimationPack {
  return id === 'custom' ? customPack : (byId(id as AnimationPackId) ?? DEFAULT_CUSTOM_ANIMATION_PACK)
}

/** 把动效包映射到根节点，所有组件通过这些 CSS 变量消费动效参数。 */
export function applyAnimationPack(
  id: AnimationPackId | string,
  customPack: AnimationPack
): void {
  const pack = resolveAnimationPack(id, customPack)
  const root = document.documentElement
  root.dataset.animPack = pack.id
  const customProperties = [
    '--anim-ease-standard',
    '--anim-ease-spring',
    '--anim-ease-smooth',
    '--anim-dur-fast',
    '--anim-dur-base',
    '--anim-dur-slow'
  ]
  if (pack.id === 'custom') {
    root.style.setProperty('--anim-ease-standard', pack.easeStandard)
    root.style.setProperty('--anim-ease-spring', pack.easeSpring)
    root.style.setProperty('--anim-ease-smooth', pack.easeSmooth)
    root.style.setProperty('--anim-dur-fast', pack.durationFast)
    root.style.setProperty('--anim-dur-base', pack.durationBase)
    root.style.setProperty('--anim-dur-slow', pack.durationSlow)
  } else {
    customProperties.forEach((property) => root.style.removeProperty(property))
  }
}
