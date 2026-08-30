/**
 * 影子对答案 · 模块注册表
 *
 * 这是唯一需要改动的文件：新增模块（R2-2 歌词 / R2-3 设置）只在这里追加一项，
 * 框架本体 shadow-compare.mjs 不需要任何修改。
 *
 * 每个模块的约定（框架只认这几个字段）：
 *   id        模块标识，命令行用 --module=<id> 选择
 *   label     中文名，报告里显示
 *   unit      被比对对象的中文量词，如「首」「行」「项」
 *   keyField  逐项对齐用的主键字段（两侧按此字段配对）
 *   fields    参与逐字段 diff 的字段列表
 *   runNode   async (input) => 数组，Node 引擎的结果
 *   runRust   async (input, native) => 数组，Rust 引擎的结果
 *   describe  (item) => string，明细里如何称呼一项
 *   compare   可选，(field, a, b) => boolean，自定义某字段的相等判定
 */

import { fileURLToPath } from 'node:url'
import { dirname, join, resolve } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '..')

/** 默认测试素材目录 */
export const DEFAULT_INPUT = join(repoRoot, 'test-assets')

/** Rust 产物所在目录（napi build 输出带平台后缀，框架会自动挑选） */
export const NATIVE_DIR = join(repoRoot, 'crates', 'aurora-native')

// ───────────────────────── scan（R2-1） ─────────────────────────

/** 与 Node 端 AUDIO_EXTENSIONS 保持一致 */
const AUDIO_EXTENSIONS = ['mp3', 'flac', 'wav', 'm4a', 'aac', 'ogg', 'opus', 'wma', 'ape']

/** 文件名回退解析，复刻主进程 parseFileName */
function parseFileNameNode(filePath, path) {
  const base = path.basename(filePath, path.extname(filePath)).trim()
  const sep = base.indexOf(' - ')
  if (sep > 0) {
    return { artist: base.slice(0, sep).trim(), title: base.slice(sep + 3).trim() }
  }
  return { title: base, artist: '' }
}

/** 递归收集音频文件，按路径排序（与 Rust 侧一致，保证配对稳定） */
async function walkAudioFiles(root, fs, path) {
  const out = []
  async function walk(dir) {
    let entries
    try {
      entries = await fs.readdir(dir, { withFileTypes: true })
    } catch {
      return // 权限等问题跳过该目录，不中断整次扫描（与 Rust 侧同语义）
    }
    for (const e of entries) {
      const full = path.join(dir, e.name)
      if (e.isDirectory()) await walk(full)
      else if (e.isFile()) {
        const ext = path.extname(e.name).slice(1).toLowerCase()
        if (AUDIO_EXTENSIONS.includes(ext)) out.push(full)
      }
    }
  }
  await walk(root)
  return out.sort()
}

export const modules = [
  {
    id: 'scan',
    label: '库扫描 / 元数据解析',
    unit: '首',
    keyField: 'path',
    fields: ['title', 'artist', 'album', 'duration', 'path'],
    describe: (item) => item?.path ?? '(未知文件)',

    /**
     * Node 侧实现。
     *
     * 刻意不 import 主进程的 local-music/index.ts —— 它引入了 electron 的 dialog，
     * 纯 Node 下加载会失败。这里复刻 parseLocalSong 的字段解析语义，
     * 并使用同一个 music-metadata 依赖，确保比对的是同一套 Node 语义。
     * 字段契约见 PROJECT_STATUS.md，改动需同步那张表。
     */
    async runNode(input) {
      const fs = await import('node:fs/promises')
      const path = await import('node:path')
      const mm = await import('music-metadata')
      const files = await walkAudioFiles(input, fs, path)
      const out = []
      for (const filePath of files) {
        const fallback = parseFileNameNode(filePath, path)
        let title = fallback.title
        let artist = fallback.artist
        let album = ''
        let duration = 0
        try {
          const meta = await mm.parseFile(filePath, { duration: true, skipCovers: true })
          if (meta.common.title?.trim()) title = meta.common.title.trim()
          const artists = meta.common.artists?.length
            ? meta.common.artists
            : [meta.common.artist ?? '']
          const joined = artists.map((s) => (s ?? '').trim()).filter(Boolean).join('、')
          if (joined) artist = joined
          album = meta.common.album?.trim() ?? ''
          duration = Math.round((meta.format.duration ?? 0) * 1000)
        } catch {
          // 标签读取失败：回退文件名 + 时长 0（与主进程 parseLocalSong 一致）
        }
        out.push({ title, artist, album, duration, path: filePath })
      }
      return out
    },

    async runRust(input, native) {
      return JSON.parse(native.scanDirectory(input))
    },

    /**
     * duration 容差：Node 走 music-metadata（浮点秒 ×1000 取整），Rust 走 lofty
     * （直接给毫秒）。两个库对同一文件的时长估算本就有帧级差异，尤其 VBR 的 mp3。
     * 影子对答案要判的是「语义是否一致」而非「位级相同」，故时长允许 ±1000ms；
     * 其余字段要求完全相等。
     */
    compare(field, a, b) {
      if (field === 'duration') {
        const x = Number(a) || 0
        const y = Number(b) || 0
        return Math.abs(x - y) <= 1000
      }
      return a === b
    },
  },

  // R2-2 在此追加 { id: 'lyrics', ... }
  // R2-3 在此追加 { id: 'settings_io', ... }
]

/** 按 id 取模块；找不到时列出可用值，避免拼错后一头雾水。 */
export function getModule(id) {
  const found = modules.find((m) => m.id === id)
  if (!found) {
    const ids = modules.map((m) => m.id).join(' / ')
    throw new Error('未注册的模块「' + id + '」。当前可用：' + ids)
  }
  return found
}