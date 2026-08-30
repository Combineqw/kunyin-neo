/**
 * 影子对答案框架（R2 核心资产）
 *
 * 同一份输入，Node 引擎算一遍、Rust 引擎算一遍，逐项逐字段 diff，
 * 输出中文人话报告。用来在真正替换实现之前，证明 Rust 侧行为与 Node 侧一致。
 *
 * 本文件是通用框架，不含任何具体模块的知识：
 * 模块的字段、算法、比对规则全部由 scripts/shadow-modules.mjs 注册。
 * 新增模块（歌词 / 设置）只改注册表，不改这里。
 *
 * 用法：
 *   node scripts/shadow-compare.mjs                     # 跑全部已注册模块
 *   node scripts/shadow-compare.mjs --module=scan       # 只跑某个模块
 *   node scripts/shadow-compare.mjs --input=D:\some\dir  # 换输入目录
 *
 * 退出码：0 = 全部 PASS，1 = 有不一致或出错（供 CI / bat 判断）。
 */

import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { createRequire } from 'node:module'

import { modules, getModule, DEFAULT_INPUT, NATIVE_DIR } from './shadow-modules.mjs'

const require = createRequire(import.meta.url)

/** 明细最多列多少条（任务书要求前 20 条） */
const MAX_DETAIL = 20

// ───────────────────────── 小工具 ─────────────────────────

function parseArgs(argv) {
  const out = { module: null, input: null }
  for (const a of argv.slice(2)) {
    const m = a.match(/^--([\w-]+)=(.*)$/)
    if (!m) continue
    if (m[1] === 'module') out.module = m[2]
    if (m[1] === 'input') out.input = m[2]
  }
  return out
}

/** 千分位，长报告里数字好读 */
const n = (x) => String(x).replace(/\B(?=(\d{3})+(?!\d))/g, ',')

/** 值转成人能看的样子：区分空串、undefined、长文本 */
function show(v) {
  if (v === undefined) return '(缺该字段)'
  if (v === null) return '(null)'
  if (v === '') return '(空)'
  const s = String(v)
  return s.length > 60 ? s.slice(0, 57) + '…' : s
}

/**
 * 加载 Rust 产物。napi build 产出的文件名带平台后缀
 * （如 aurora-native.win32-x64-msvc.node），所以按目录里的实际文件挑。
 */
function loadNative() {
  if (!existsSync(NATIVE_DIR)) {
    throw new Error('找不到原生模块目录：' + NATIVE_DIR + '\n请先运行 npm run native:build')
  }
  const candidates = readdirSync(NATIVE_DIR).filter((f) => f.endsWith('.node'))
  if (!candidates.length) {
    throw new Error(
      '原生模块尚未编译（' + NATIVE_DIR + ' 下没有 .node 文件）\n请先运行 npm run native:build'
    )
  }
  // 优先当前平台后缀，其次任意一个
  const preferred =
    candidates.find((f) => f.includes(process.platform) && f.includes(process.arch)) ??
    candidates[0]
  return require(join(NATIVE_DIR, preferred))
}

// ───────────────────────── 比对核心 ─────────────────────────

/**
 * 逐项逐字段 diff。
 *
 * 两侧先按 keyField 建索引再配对，因此不受各自遍历顺序影响；
 * 只在一侧出现的项单独归类（缺失/多余），不会被算成「字段不一致」。
 */
function diff(mod, nodeItems, rustItems) {
  const key = mod.keyField
  const eq = mod.compare ?? ((_f, a, b) => a === b)

  const nodeMap = new Map(nodeItems.map((it) => [it?.[key], it]))
  const rustMap = new Map(rustItems.map((it) => [it?.[key], it]))

  const onlyNode = []
  const onlyRust = []
  const mismatches = []
  let matched = 0

  for (const [k, nodeItem] of nodeMap) {
    const rustItem = rustMap.get(k)
    if (!rustItem) {
      onlyNode.push(nodeItem)
      continue
    }
    const bad = []
    for (const field of mod.fields) {
      const a = nodeItem?.[field]
      const b = rustItem?.[field]
      if (!eq(field, a, b)) bad.push({ field, node: a, rust: b })
    }
    if (bad.length) mismatches.push({ item: nodeItem, fields: bad })
    else matched += 1
  }
  for (const [k, rustItem] of rustMap) {
    if (!nodeMap.has(k)) onlyRust.push(rustItem)
  }

  return { matched, mismatches, onlyNode, onlyRust }
}

/** 跑单个模块，返回结果对象；自身不打印结论，交给上层统一排版 */
async function runModule(mod, input, native) {
  const t0 = Date.now()
  const nodeItems = await mod.runNode(input)
  const t1 = Date.now()
  const rustItems = await mod.runRust(input, native)
  const t2 = Date.now()

  if (!Array.isArray(nodeItems) || !Array.isArray(rustItems)) {
    throw new Error('模块「' + mod.id + '」的 runNode/runRust 必须返回数组')
  }

  return {
    mod,
    nodeCount: nodeItems.length,
    rustCount: rustItems.length,
    nodeMs: t1 - t0,
    rustMs: t2 - t1,
    ...diff(mod, nodeItems, rustItems),
  }
}

// ───────────────────────── 报告排版 ─────────────────────────

function printReport(r) {
  const u = r.mod.unit
  const total = r.nodeCount
  const bad = r.mismatches.length + r.onlyNode.length + r.onlyRust.length
  const pass = bad === 0 && total > 0

  console.log('')
  console.log('─'.repeat(64))
  console.log('模块：' + r.mod.label + '（' + r.mod.id + '）')
  console.log('─'.repeat(64))
  console.log('  Node 引擎：' + n(r.nodeCount) + ' ' + u + '，耗时 ' + n(r.nodeMs) + ' ms')
  console.log('  Rust 引擎：' + n(r.rustCount) + ' ' + u + '，耗时 ' + n(r.rustMs) + ' ms')
  if (r.nodeMs > 0 && r.rustMs >= 0) {
    const speed = r.rustMs === 0 ? '∞' : (r.nodeMs / r.rustMs).toFixed(1)
    console.log('  速度对比：Rust 约为 Node 的 ' + speed + ' 倍')
  }
  console.log('')

  if (r.onlyNode.length) {
    console.log('  ⚠ 只有 Node 扫到、Rust 没扫到：' + n(r.onlyNode.length) + ' ' + u)
    for (const it of r.onlyNode.slice(0, MAX_DETAIL)) {
      console.log('      · ' + r.mod.describe(it))
    }
  }
  if (r.onlyRust.length) {
    console.log('  ⚠ 只有 Rust 扫到、Node 没扫到：' + n(r.onlyRust.length) + ' ' + u)
    for (const it of r.onlyRust.slice(0, MAX_DETAIL)) {
      console.log('      · ' + r.mod.describe(it))
    }
  }

  if (r.mismatches.length) {
    console.log('  ✗ 字段不一致明细（最多列 ' + MAX_DETAIL + ' 条）：')
    for (const m of r.mismatches.slice(0, MAX_DETAIL)) {
      console.log('')
      console.log('    ' + r.mod.describe(m.item))
      for (const f of m.fields) {
        console.log('        字段「' + f.field + '」')
        console.log('            Node：' + show(f.node))
        console.log('            Rust：' + show(f.rust))
      }
    }
    if (r.mismatches.length > MAX_DETAIL) {
      console.log('')
      console.log('    …另有 ' + n(r.mismatches.length - MAX_DETAIL) + ' 条未列出')
    }
    console.log('')
  }

  const verdict = pass
    ? 'PASS'
    : total === 0
      ? 'PASS（无素材，未做实质比对）'
      : 'FAIL'
  console.log(
    '  结论：共 ' + n(total) + ' ' + u +
      ' · 一致 ' + n(r.matched) + ' · 不一致 ' + n(bad) +
      ' → ' + verdict
  )
  return pass || total === 0
}

// ───────────────────────── 入口 ─────────────────────────

async function main() {
  const args = parseArgs(process.argv)
  const input = args.input ?? DEFAULT_INPUT

  console.log('')
  console.log('══════════════════════════════════════════════════════════════')
  console.log('  坤音 neo · Rust 影子对答案')
  console.log('══════════════════════════════════════════════════════════════')
  console.log('  测试素材目录：' + input)

  if (!existsSync(input)) {
    console.log('')
    console.log('  ✗ 素材目录不存在。请创建 test-assets/ 并放入音乐文件后重试。')
    process.exitCode = 1
    return
  }

  let native
  try {
    native = loadNative()
  } catch (e) {
    console.log('')
    console.log('  ✗ ' + e.message)
    process.exitCode = 1
    return
  }
  const ver = typeof native.nativeVersion === 'function' ? native.nativeVersion() : '未知'
  console.log('  原生模块版本：' + ver)

  const targets = args.module ? [getModule(args.module)] : modules
  console.log('  参与比对的模块：' + targets.map((m) => m.id).join(' / '))

  let allPass = true
  for (const mod of targets) {
    try {
      const r = await runModule(mod, input, native)
      const ok = printReport(r)
      if (!ok) allPass = false
    } catch (e) {
      console.log('')
      console.log('  ✗ 模块「' + mod.id + '」执行出错：' + e.message)
      allPass = false
    }
  }

  console.log('')
  console.log('══════════════════════════════════════════════════════════════')
  console.log(allPass ? '  总结论：全部 PASS ✅' : '  总结论：存在不一致 ❌')
  console.log('══════════════════════════════════════════════════════════════')
  console.log('')
  process.exitCode = allPass ? 0 : 1
}

main().catch((e) => {
  console.error('')
  console.error('  ✗ 框架自身出错：' + (e?.stack ?? e))
  process.exitCode = 1
})