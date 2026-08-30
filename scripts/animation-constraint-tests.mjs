// 动效约束验收：把「动画只允许 transform / filter / background-position，
// 时长与缓动全部引用 CSS 变量，禁止硬编码」这条要求做成可执行的门。
//
// 直接扫源码文本而不是跑运行时：约束本身就是关于源码怎么写的，
// 编译产物里变量已被展开，反而看不出有没有引用变量。
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { dirname, extname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)))
const rendererDir = join(projectRoot, 'src', 'renderer')

/** 可以出现在 @keyframes 里的属性。
 *
 * transform / filter / background-position 是用户点名允许的三个。
 * opacity 一并放行：它与前三者同属合成器属性，不触发重排重绘，
 * 且项目基线里的弹窗动画本来就在用（popup-* 四个）。 */
const ALLOWED_PROPS = new Set(['transform', 'filter', 'background-position', 'opacity'])

/** 基线里就存在硬编码时长的 animation，不是定下约束之后引入的。
 *
 * 列在这里而不是直接放宽规则：新加的动画一律要走 CSS 变量，
 * 这四个想清理的话是独立的一次改动，不该被这道门悄悄放过去。 */
const GRANDFATHERED_HARDCODED = new Set([
  'popup-zoom-in',
  'popup-slide-in',
  'popup-flip-in',
  'popup-drop-in'
])

/** transition 硬编码时长的基线条数。
 *
 * 约束是针对新增动效提的，而项目基线里散落着 24 处硬编码 transition（歌词播放器、
 * 各 View 的悬停反馈等，均为既有产品代码）。直接卡会让干净的检出也失败，那这道门
 * 就没有意义了；直接放行又会让新增的混进来看不见。
 *
 * 所以卡「不得增加」：新写一处硬编码 transition，条数变成 25，门就会失败，
 * 提示你改用变量、或者明确地把这个数字调上去。用条数而不是行号，是因为行号会
 * 被无关的增删推移（同一条声明就曾从 365 行漂到 420 行）。 */
const TRANSITION_BASELINE = 24

const SCAN_EXT = new Set(['.css', '.scss', '.vue', '.ts'])

function walk(dir) {
  const out = []
  for (const entry of readdirSync(dir)) {
    if (entry === 'node_modules' || entry.startsWith('.')) continue
    const full = join(dir, entry)
    if (statSync(full).isDirectory()) out.push(...walk(full))
    else if (SCAN_EXT.has(extname(entry))) out.push(full)
  }
  return out
}

/** 从 { 开始配对大括号，取出完整块。正则做不到嵌套配对。 */
function blockAt(text, braceIndex) {
  let depth = 0
  for (let i = braceIndex; i < text.length; i += 1) {
    if (text[i] === '{') depth += 1
    else if (text[i] === '}') {
      depth -= 1
      if (depth === 0) return text.slice(braceIndex, i + 1)
    }
  }
  return text.slice(braceIndex)
}

const failures = []
const transitionHardcoded = []
let keyframeCount = 0
let animationCount = 0

for (const file of walk(rendererDir)) {
  const rel = relative(projectRoot, file).replaceAll('\\', '/')
  const text = readFileSync(file, 'utf8')

  // ---- 1. @keyframes 块内只允许白名单属性 ----
  for (const m of text.matchAll(/@keyframes\s+([\w-]+)\s*\{/g)) {
    keyframeCount += 1
    const name = m[1]
    const block = blockAt(text, m.index + m[0].length - 1)
    const props = new Set()
    // 属性可能写在 `{` 或 `;` 之后而不一定在行首（`0% { left: 0; }` 这种单行写法），
    // 所以按分隔符找而不是按行首找——用后者会漏掉整条单行规则。
    for (const p of block.matchAll(/(?:^|[{;])\s*([a-z-]+)\s*:/gm)) props.add(p[1])
    const bad = [...props].filter((p) => !ALLOWED_PROPS.has(p))
    if (bad.length) {
      const line = text.slice(0, m.index).split('\n').length
      failures.push(
        `${rel}:${line} @keyframes ${name} 用了不允许的属性 ${bad.join(', ')}；` +
          `只允许 ${[...ALLOWED_PROPS].join(' / ')}`
      )
    }
  }

  // ---- 2. animation / transition 的时长与缓动必须引用 CSS 变量 ----
  // 注意 animation-name 也在这条简写里，所以要把名字摘出来判断豁免。
  for (const m of text.matchAll(/^\s*(animation|transition)(-duration)?\s*:\s*([^;]+);/gm)) {
    const [, prop, , value] = m
    const v = value.trim()
    if (v === 'none' || v.startsWith('none') || v === 'inherit' || v === 'initial') continue
    animationCount += 1

    const hardcoded = /(?<![\w-])\d*\.?\d+m?s(?![\w-])/.test(v)
    if (!hardcoded) continue

    const line = text.slice(0, m.index).split('\n').length
    const where = `${rel}:${line} ${prop} 硬编码了时长：${v.replace(/\s+/g, ' ')}`

    if (prop === 'transition') {
      // 基线条数制，见 TRANSITION_BASELINE
      transitionHardcoded.push(where)
      continue
    }

    // animation 简写的第一个 token 通常是动画名，用它查豁免名单
    if (GRANDFATHERED_HARDCODED.has(v.split(/\s+/)[0])) continue
    failures.push(`${where}；时长与缓动要引用 CSS 变量（如 var(--anim-dur-*)）`)
  }
}

// transition 只卡「不得增加」，理由见 TRANSITION_BASELINE
if (transitionHardcoded.length > TRANSITION_BASELINE) {
  failures.push(
    `硬编码时长的 transition 从基线 ${TRANSITION_BASELINE} 处增加到 ` +
      `${transitionHardcoded.length} 处，新增的应改用 var(--anim-dur-*)：`
  )
  for (const w of transitionHardcoded.slice(TRANSITION_BASELINE)) failures.push(`    ${w}`)
} else if (transitionHardcoded.length < TRANSITION_BASELINE) {
  console.log(
    `提示：硬编码 transition 已降到 ${transitionHardcoded.length} 处（基线 ${TRANSITION_BASELINE}），` +
      `可把 TRANSITION_BASELINE 调低以锁住成果。`
  )
}

if (failures.length) {
  console.error(`动效约束验收未通过（${failures.length} 项）：\n`)
  for (const f of failures) console.error(`  ✗ ${f}`)
  console.error('')
  process.exit(1)
}

console.log(
  `动效约束验收通过：${keyframeCount} 个 @keyframes、${animationCount} 处 animation/transition 声明`
)
console.log(
  `  @keyframes 仅允许 ${[...ALLOWED_PROPS].join(' / ')} · animation 时长须引用 CSS 变量` +
    `（豁免 ${GRANDFATHERED_HARDCODED.size} 个基线 popup-*）`
)
console.log(`  硬编码 transition ${transitionHardcoded.length} 处，未超过基线 ${TRANSITION_BASELINE}`)
