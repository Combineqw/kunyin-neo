/**
 * 从本机真实数据导出歌词测试素材到 test-assets/lyrics/。
 *
 * 两个来源，都是只读：
 *   1. %APPDATA%/kunyin-desktop/data/cache/lyric-cache.json —— 平时听歌攒下的
 *      歌词缓存（LRU 结构 [[key, entry], ...]，正文在 entry.lyric.lrc）
 *   2. 用户音乐文件夹里已有的 .lrc 文件
 *
 * 缓存原文件绝不修改。已存在的同名素材默认跳过（幂等），--force 覆盖。
 *
 * 用法：node scripts/export-lyrics.mjs [--force]
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, copyFileSync } from 'node:fs'
import { join, resolve, dirname, basename } from 'node:path'
import { homedir } from 'node:os'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '..')
const outDir = join(repoRoot, 'test-assets', 'lyrics')
const force = process.argv.includes('--force')

/** 文件名里不能出现的字符 → 下划线 */
const safeName = (s) => s.replace(/[\\/:*?"<>|]/g, '_').slice(0, 120)

mkdirSync(outDir, { recursive: true })

let written = 0
let skipped = 0

// ── 来源 1：歌词缓存 ──
const cachePath = join(
  process.env.APPDATA ?? join(homedir(), 'AppData', 'Roaming'),
  'kunyin-desktop', 'data', 'cache', 'lyric-cache.json'
)

if (existsSync(cachePath)) {
  const raw = JSON.parse(readFileSync(cachePath, 'utf8'))
  const pairs = Array.isArray(raw) ? raw : Object.entries(raw)
  console.log('歌词缓存：' + pairs.length + ' 条')

  for (const pair of pairs) {
    const [key, entry] = Array.isArray(pair) ? pair : [null, pair]
    const text = entry?.lyric?.lrc
    if (!text || !text.trim()) continue
    const target = join(outDir, safeName(String(key)) + '.lrc')
    if (existsSync(target) && !force) { skipped += 1; continue }
    writeFileSync(target, text, 'utf8')
    written += 1
  }
} else {
  console.log('未找到歌词缓存：' + cachePath)
}

// ── 来源 2：音乐文件夹里现成的 .lrc ──
const musicRoots = [join(homedir(), 'Music')]

function collectLrc(dir, depth = 0) {
  if (depth > 4 || !existsSync(dir)) return []
  const out = []
  let entries
  try {
    entries = readdirSync(dir, { withFileTypes: true })
  } catch {
    return out // 权限问题跳过
  }
  for (const e of entries) {
    const full = join(dir, e.name)
    if (e.isDirectory()) out.push(...collectLrc(full, depth + 1))
    else if (e.name.toLowerCase().endsWith('.lrc')) out.push(full)
  }
  return out
}

for (const root of musicRoots) {
  const found = collectLrc(root)
  if (found.length) console.log('音乐文件夹 .lrc：' + found.length + ' 个')
  for (const src of found) {
    const target = join(outDir, safeName(basename(src)))
    if (existsSync(target) && !force) { skipped += 1; continue }
    copyFileSync(src, target)
    written += 1
  }
}

const total = readdirSync(outDir).filter((f) => f.toLowerCase().endsWith('.lrc')).length
console.log('')
console.log('导出完成：新增 ' + written + ' 个，跳过 ' + skipped + ' 个（已存在）')
console.log('素材目录共 ' + total + ' 个 .lrc → ' + outDir)