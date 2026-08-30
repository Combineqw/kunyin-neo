/**
 * 一键测试入口（供 scripts/native-test.bat 双击调用）。
 *
 * 为什么中文提示放在这里而不是 bat 里：
 * cmd 解析 .bat 文件内容用的是系统 ANSI 代码页（简中机器上是 GBK），
 * 而 Node 的 stdout 是 UTF-8。同一个控制台没法用一个代码页同时正确显示两者——
 * bat 里写中文就得 GBK 存盘，那 Node 的输出又会乱码，反之亦然。
 * 所以 bat 保持纯 ASCII 只做 chcp + 调用，全部人话由 Node 统一打印。
 */

import { spawnSync } from 'node:child_process'
import { existsSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '..')
const assets = join(repoRoot, 'test-assets')

const line = (s = '') => console.log(s)
const rule = (ch = '=') => line(ch.repeat(62))

/** 在仓库根跑一个 node 脚本，输出直通控制台 */
function runNode(script) {
  const r = spawnSync(process.execPath, [join('scripts', script)], {
    cwd: repoRoot,
    stdio: 'inherit',
  })
  return r.status ?? 1
}

line()
rule()
line('   坤音 neo · Rust 原生模块 对答案测试')
rule()
line()
line('  这个测试在做什么：')
line('  让两套引擎分别扫描同一批音乐 —— 一套是现在正在用的 Node 引擎，')
line('  一套是新写的 Rust 引擎 —— 然后逐首逐字段核对，确认新引擎的结果')
line('  和老引擎完全一样。一致才说明新引擎可以放心替换。')
line()

// 素材自检：先把「没放歌」这种最常见的情况讲清楚，别让人对着空报告猜
if (!existsSync(assets)) {
  line('  ✗ 找不到测试素材文件夹：')
  line('    ' + assets)
  line('    请新建这个文件夹，放几十首自己的歌（mp3 / flac / ogg 都行）后重试。')
  line()
  process.exit(1)
}

const AUDIO = ['.mp3', '.flac', '.ogg', '.m4a', '.wav', '.wma', '.aac', '.opus', '.ape']
function countAudio(dir) {
  let n = 0
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.isDirectory()) n += countAudio(join(dir, e.name))
    else if (AUDIO.some((x) => e.name.toLowerCase().endsWith(x))) n += 1
  }
  return n
}
const total = countAudio(assets)
line('  测试素材：' + assets)
line('  发现音频 ' + total + ' 首')
if (total === 0) {
  line()
  line('  ✗ 这个文件夹里没有音频文件，无法对答案。')
  line('    请放几十首自己的歌（mp3 / flac / ogg 都行）后重试。')
  line()
  process.exit(1)
}
if (total < 50) {
  line('  提示：验收标准建议放 50 首以上，样本越多越能暴露差异。')
}
line()

rule('-')
line('  第 1 步 / 共 2 步：编译 Rust 模块')
line('  （第一次编译较慢，之后有缓存会快很多）')
rule('-')
line()
const buildCode = runNode('native-build.mjs')
if (buildCode !== 0) {
  line()
  rule()
  line('  ✗ 编译失败')
  rule()
  line('  最常见的原因是没装 Rust 工具链，装一下即可：https://rustup.rs')
  line('  装完重新双击本测试，不需要手动配置环境变量。')
  line('  如果已经装过，请把上面的报错内容截图发给开发者。')
  line()
  process.exit(buildCode)
}

line()
rule('-')
line('  第 2 步 / 共 2 步：两套引擎对答案')
rule('-')
const cmpCode = runNode('shadow-compare.mjs')

line()
rule()
if (cmpCode === 0) {
  line('  ✓ 测试通过：两套引擎的结果完全一致')
  line()
  line('  可以顺手再抽查一下：随便挑两首歌，右键 → 属性 → 详细信息，')
  line('  看歌名 / 歌手 / 专辑跟上面报告里的是否对得上。')
} else {
  line('  ✗ 对答案未通过：两套引擎的结果有出入')
  line()
  line('  上面的报告已经列出具体是哪首歌、哪个字段不一样、两边各是什么。')
  line('  请把报告截图发给开发者 —— 这正是这个测试要抓的问题。')
}
rule()
process.exit(cmpCode)