/**
 * 按压弹回的触发器。
 *
 * 为什么需要 JS：过冲必须烘焙在 @keyframes 里（写在 --anim-ease-spring 的控制点上
 * 会被动效包覆盖，OriginOS 包的曲线峰值恰好 1.000，弹性直接归零）。而 CSS 没有
 * "退出 :active 时播放动画"的选择器——只写 .pressable{animation:...} 的话，
 * 首屏加载时页面上所有可按元件会一起抖一遍。
 *
 * 所以用一个全局委托监听器：pointerup / pointercancel 时给元件加类，
 * 动画结束（或超时兜底）摘掉。全窗口仅 3 个监听器，不随元件数量增长。
 */

const REBOUND_CLASS = 'press-rebound'

/** 记录已挂上的一次性清理函数，避免连击时重复绑定 animationend */
const pending = new WeakMap<Element, () => void>()

function isDisabled(el: Element): boolean {
  if (el.classList.contains('disabled')) return true
  return 'disabled' in el && Boolean((el as HTMLButtonElement).disabled)
}

function trigger(target: EventTarget | null): void {
  if (!(target instanceof Element)) return
  const el = target.closest('.pressable')
  if (!el || isDisabled(el)) return

  // 连击：先把上一轮收干净，再重新起一轮。否则第二次按下时类还在，
  // 浏览器不会重启动画（同名同类，无变化就不重放）。
  pending.get(el)?.()

  // 强制重排让浏览器认到"类被摘掉过"，连击才会每次都重新播放。
  // 读 offsetWidth 是最便宜的同步布局刷新方式。
  el.classList.remove(REBOUND_CLASS)
  void (el as HTMLElement).offsetWidth
  el.classList.add(REBOUND_CLASS)

  let timer = 0
  const cleanup = (): void => {
    window.clearTimeout(timer)
    el.removeEventListener('animationend', onEnd)
    el.classList.remove(REBOUND_CLASS)
    pending.delete(el)
  }
  function onEnd(e: Event): void {
    // 子元件自己的动画冒泡上来时不要误摘
    if ((e as AnimationEvent).animationName !== 'press-rebound') return
    cleanup()
  }
  el.addEventListener('animationend', onEnd)
  // 兜底：动画被 disable-animation / prefers-reduced-motion 停掉时
  // animationend 永远不来，类会永久残留。350ms 略大于 --anim-dur-rebound。
  timer = window.setTimeout(cleanup, 600)
  pending.set(el, cleanup)
}

let installed = false

/** 全局安装一次。重复调用无副作用。 */
export function installPressFeedback(): void {
  if (installed) return
  installed = true
  // 捕获阶段：元件自己 stopPropagation 时（如 SongRow 的 @click.stop 系操作按钮）
  // 冒泡阶段可能收不到，捕获阶段一定能收到。
  const opts: AddEventListenerOptions = { capture: true, passive: true }
  window.addEventListener('pointerup', (e) => trigger(e.target), opts)
  window.addEventListener('pointercancel', (e) => trigger(e.target), opts)
  // 键盘激活（Enter/Space）不产生 pointer 事件，补一条 keyup
  window.addEventListener(
    'keyup',
    (e) => {
      if (e.key === 'Enter' || e.key === ' ') trigger(e.target)
    },
    opts
  )
}
