/**
 * 收歌脚本：从系统「音乐」文件夹复制真实歌曲到测试素材目录，并生成清单。
 *
 * 边界（硬约束）：
 *   · 只读来源目录，只写 test-assets/songs
 *   · 禁止全盘扫描 —— 只看来源目录及其子目录，绝不向上或旁路搜索
 *   · 来源默认系统音乐文件夹；--source 可显式指定单一目录，
 *     但必须是已存在的目录，且仍只递归它自己（不放宽扫描范围）
 *   · 不足 50 首如实报告，绝不造假凑数（不生成假文件、不拿音效冒充）
 *
 * 产出 test-assets/songs-manifest.json 入库：以后对数认清单不认文件夹。
 *
 * 用法：node scripts/collect-songs.mjs [--target=50] [--force] [--source=<目录>]
 */

import { readdirSync, statSync, copyFileSync, mkdirSync, existsSync, writeFileSync, readFileSync } from 'node:fs'
import { join, resolve, dirname, basename, extname, relative } from 'node:path'
import { homedir } from 'node:os'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '..')
const outDir = join(repoRoot, 'test-assets', 'songs')
const manifestPath = join(repoRoot, 'test-assets', 'songs-manifest.json')

/** 默认来源：系统音乐文件夹。 */
const DEFAULT_MUSIC_ROOT = join(homedir(), 'Music')

const AUDIO_EXT = ['.mp3', '.flac', '.wav', '.m4a', '.aac', '.ogg', '.opus', '.wma', '.ape']

const argOf = (name, fallback) => {
  const hit = process.argv.find((a) => a.startsWith('--' + name + '='))
  return hit ? hit.slice(name.length + 3) : fallback
}
const TARGET = Number(argOf('target', '50')) || 50
const force = process.argv.includes('--force')

/**
 * 来源目录：默认系统音乐文件夹，可用 --source=<目录> 显式改写。
 *
 * 仍然只递归这一个目录 —— --source 是「换一个起点」，不是「放开扫描范围」。
 */
const MUSIC_ROOT = resolve(argOf('source', DEFAULT_MUSIC_ROOT))
const usingCustomSource = MUSIC_ROOT !== resolve(DEFAULT_MUSIC_ROOT)

const line = (s = '') => console.log(s)
const rule = (c = '=') => line(c.repeat(62))

line()
rule()
line('   坤音 neo · 收集测试歌曲')
rule()
line()
line('  从来源目录复制真实歌曲到测试素材目录。')
line('  只读来源目录、只写测试目录，不做全盘扫描。')
line()
line('  来源目录：  ' + MUSIC_ROOT + (usingCustomSource ? '（--source 指定）' : '（系统音乐文件夹）'))
line('  测试目录：  ' + outDir)
line('  目标数量：  ' + TARGET + ' 首')
line()

if (!existsSync(MUSIC_ROOT)) {
  line('  ✗ 找不到来源目录：' + MUSIC_ROOT)
  line('    请确认该目录存在后重试；也可用 --source=<目录> 指定其它来源。')
  line()
  process.exit(1)
}

/** 递归收集音频（仅限音乐文件夹内） */
function collect(dir, depth = 0) {
  if (depth > 8) return []
  const out = []
  let entries
  try {
    entries = readdirSync(dir, { withFileTypes: true })
  } catch {
    return out // 权限问题跳过该目录
  }
  for (const e of entries) {
    const full = join(dir, e.name)
    if (e.isDirectory()) out.push(...collect(full, depth + 1))
    else if (AUDIO_EXT.includes(extname(e.name).toLowerCase())) out.push(full)
  }
  return out
}

const found = collect(MUSIC_ROOT).sort()
line('  扫描结果：找到 ' + found.length + ' 首音频')

if (found.length === 0) {
  line()
  line('  ✗ 来源目录里没有音频文件，无法收集。')
  line('    请往 ' + MUSIC_ROOT + ' 放入歌曲后重试。')
  line()
  process.exit(1)
}

// 按扩展名分布报告
const byExt = new Map()
for (const f of found) {
  const k = extname(f).toLowerCase()
  byExt.set(k, (byExt.get(k) ?? 0) + 1)
}
line('  格式分布：' + [...byExt.entries()].sort((a, b) => b[1] - a[1]).map(([k, v]) => k + ' × ' + v).join('，'))
line()

mkdirSync(outDir, { recursive: true })

let copied = 0
let skipped = 0
const entries = []

for (const src of found) {
  // 保留相对路径结构，避免不同专辑同名文件互相覆盖
  const rel = relative(MUSIC_ROOT, src).replace(/[\\/]/g, '__')
  const target = join(outDir, rel)
  if (existsSync(target) && !force) {
    skipped += 1
  } else {
    copyFileSync(src, target)   // 只写测试目录
    copied += 1
  }
  const buf = readFileSync(target)
  entries.push({
    name: basename(target),
    ext: extname(target).toLowerCase(),
    bytes: buf.length,
    sha256: createHash('sha256').update(buf).digest('hex'),
  })
}

entries.sort((a, b) => a.name.localeCompare(b.name))

// 生成清单：文件数 / 扩展名分布 / 逐文件 名称+大小+SHA-256
const extDist = {}
for (const e of entries) extDist[e.ext] = (extDist[e.ext] ?? 0) + 1
const manifest = {
  generatedAt: new Date().toISOString(),
  source: MUSIC_ROOT,
  sourceIsCustom: usingCustomSource,
  targetCount: TARGET,
  actualCount: entries.length,
  meetsTarget: entries.length >= TARGET,
  totalBytes: entries.reduce((s, e) => s + e.bytes, 0),
  extDistribution: extDist,
  files: entries,
}
writeFileSync(manifestPath, JSON.stringify(manifest, null, 2), 'utf8')

line('  复制完成：新增 ' + copied + ' 首，跳过 ' + skipped + ' 首（已存在）')
line('  清单已生成：' + manifestPath)
line()
rule('-')
if (manifest.meetsTarget) {
  line('  ✓ 素材达标：共 ' + entries.length + ' 首（目标 ' + TARGET + ' 首）')
} else {
  const gap = TARGET - entries.length
  line('  ⚠ 素材不足：共 ' + entries.length + ' 首，距目标 ' + TARGET + ' 首还差 ' + gap + ' 首')
  line()
  line('  这是如实报告，不会用生成的假文件或游戏音效凑数。')
  line('  请往下面这个目录再放 ' + gap + ' 首歌，然后重新运行本脚本：')
  line('      ' + MUSIC_ROOT)
}
rule('-')
line()
// 不足目标时以非零退出码收尾，便于上层流程判断
process.exit(manifest.meetsTarget ? 0 : 2)