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
import { statSync } from 'node:fs'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '..')

/** 默认测试素材目录 */
export const DEFAULT_INPUT = join(repoRoot, 'test-assets')

/** Rust 产物所在目录（napi build 输出带平台后缀，框架会自动挑选） */
export const NATIVE_DIR = join(repoRoot, 'crates', 'aurora-native')

// ─────────────── 真实数据位置（settings_io 用，只读 + 防篡改校验） ───────────────

/** userData 目录：Electron 的 app.getPath('userData') 在纯 Node 下的等价推导 */
const userData = join(
  process.env.APPDATA ?? join(process.env.USERPROFILE ?? '', 'AppData', 'Roaming'),
  'kunyin-desktop'
)

/** 真实设置文件；不存在时 settings_io 模块自动跳过（不报错） */
export const REAL_SETTINGS = (() => {
  const p = join(userData, 'data', 'settings.json')
  return existsSyncSafe(p) ? p : null
})()

/** 真实歌单库（SQLite）；一并纳入防篡改校验，确保写测试没波及它 */
export const REAL_DB = (() => {
  const p = join(userData, 'data', 'kunyin_music.db')
  return existsSyncSafe(p) ? p : null
})()

function existsSyncSafe(p) {
  try {
    // 用 statSync 而非 existsSync，避免把权限错误误判为「文件不存在」
    statSync(p)
    return true
  } catch {
    return false
  }
}

/**
 * 回环测试用例：读 → 改一个键 → 序列化 → 读回。
 *
 * 覆盖几类最容易出分歧的值：浮点、整数、布尔、含非 ASCII 的字符串、
 * 数组整体替换、嵌套新键。
 */
export const LOOP_CASES = [
  { id: 'float', keyPath: 'player.volume', value: 0.4321 },
  { id: 'negative-float', keyPath: 'player.equalizerProfile.preampDb', value: -12.75 },
  { id: 'integer', keyPath: 'player.srsIntensity', value: 42 },
  { id: 'zero', keyPath: 'player.srsBass', value: 0 },
  { id: 'boolean', keyPath: 'player.srsEnabled', value: true },
  { id: 'unicode', keyPath: 'appearance.themeId', value: '极光·美人鱼「测试」' },
  { id: 'escape', keyPath: 'behavior.customAnimationPack.name', value: 'a"b\\c\nd\te' },
  { id: 'array-replace', keyPath: 'player.equalizerFilters', value: [] },
  { id: 'nested-new', keyPath: 'network.proxy.host', value: '127.0.0.1' },
  { id: 'exponent', keyPath: 'player.equalizerPreampDb', value: 1e-7 },
]

/** 按点分路径写入一个键，中间层缺失则创建（对齐 Rust 侧 set_by_path） */
export function setByPath(root, keyPath, value) {
  const parts = keyPath.split('.').filter(Boolean)
  let cur = root
  for (let i = 0; i < parts.length - 1; i++) {
    const k = parts[i]
    if (cur[k] === null || typeof cur[k] !== 'object' || Array.isArray(cur[k])) cur[k] = {}
    cur = cur[k]
  }
  cur[parts[parts.length - 1]] = value
}

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
    // 能力位：纯只读，无写路径
    capabilities: { read: true, write: false },
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
      // Rust 侧返回 { tracks, skippedNonAudio, parseFailed, walkErrors }。
      // 计数存到 this.stats 供框架展示；比对仍只针对 tracks。
      const r = JSON.parse(native.scanDirectory(input))
      this.stats = {
        skippedNonAudio: r.skippedNonAudio,
        parseFailed: r.parseFailed,
        walkErrors: r.walkErrors,
      }
      return r.tracks
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

  // ───────────────────────── lyrics（R2-2） ─────────────────────────
  {
    id: 'lyrics',
    label: '歌词解析（LRC 行级）',
    // 能力位：纯只读，无写路径
    capabilities: { read: true, write: false },
    unit: '行',
    // 一首歌一个文件，但比对粒度是「行」：键取 文件路径#行号
    keyField: 'key',
    fields: ['start', 'end', 'text'],
    describe: (it) => it?.key ?? '(未知行)',

    /**
     * Node 侧实现。
     *
     * 同样不 import 渲染层的 ParserPipeline —— 那条链路带 Vue 依赖、
     * 且会跑 8 个 transform 插件。这里复刻 matchLyric + processNormal 的
     * **行级**语义（本块范围），与 Rust 侧一一对应。
     * 契约见 PROJECT_STATUS.md「lyrics 模块」表。
     */
    async runNode(input) {
      const fs = await import('node:fs/promises')
      const path = await import('node:path')

      // 与 kit/utils/time/index.ts 的 parseMilliSecond 一致：截断而非四舍五入
      const parseMilli = (frac) => parseInt(frac.padEnd(3, '0').slice(0, 3), 10) || 0

      // 对齐 kit/utils/time/index.ts 的 parseTime
      const parseTime = (content) => {
        const t = (content ?? '').trim()
        if (!t) return null
        if (t.startsWith('.')) {
          const v = t.slice(1)
          return /^\d+$/.test(v) ? parseMilli(v) : null
        }
        if (/^\d+$/.test(t)) return parseInt(t, 10)
        const m = t.match(/^(?:(?:(\d+):)?(\d+):)?(\d+)(?:\.(\d+))?$/u)
        if (!m) return null
        const h = parseInt(m[1], 10) || 0
        const mi = parseInt(m[2], 10) || 0
        const s = parseInt(m[3], 10) || 0
        const ms = parseMilli(m[4] || '0')
        return ((h * 60 + mi) * 60 + s) * 1000 + ms
      }

      const parseTagTime = (tag) => {
        const c = (tag ?? '').trim().match(/^[<\[]([^>\]]+)[>\]]$/)
        if (!c) return null
        const v = c[1]?.trim()
        return v ? parseTime(v) : null
      }

      // 对齐 kit/plugin-format-lrc/parser/utils/match.ts
      const LINE_REGEXP =
        /(\[(?:[a-zA-Z]+\s*:\s*[^\]]+|(?:\d+:)?\d+:\d+(?:\.\d+)?)\])([\s\S]*?)(?=(?:\[(?:[a-zA-Z]+\s*:\s*[^\]]+|(?:\d+:)?\d+:\d+(?:\.\d+)?)\])|$)/g
      const META_REGEX = /^\[[a-zA-Z]+:[^\]]+\]$/
      const LINE_REGEX = /^\[(\d+:)?\d+:\d+(\.\d+)?\].+$/
      const rmSpaceAll = (s) => s.replaceAll(/\s+/g, '').trim()

      const parseLrc = (content) => {
        // BOM 会让首个标签匹配失败
        let text = content
        if (text.charCodeAt(0) === 0xfeff) text = text.slice(1)
        if (!text.trim()) return []

        const lines = []
        for (const src of text.split('\n')) {
          if (!src.trim()) continue
          for (const m of src.matchAll(LINE_REGEXP)) {
            const raw = m[0]
            const tag = (m[1] || '').trim()
            const body = (m[2] || '').trim()
            if (!tag) continue
            const flat = rmSpaceAll(raw)
            if (META_REGEX.test(flat)) continue // meta 行不算歌词
            if (!LINE_REGEX.test(flat)) continue // 无正文的行被丢弃
            lines.push({ start: parseTagTime(tag) || 0, end: 0, text: body })
          }
        }

        // end 用下一行 start 回填；末行保持 0
        for (let i = 0; i < lines.length - 1; i++) lines[i].end = lines[i + 1].start
        return lines
      }

      // 收集 .lrc 并按路径排序（与 Rust 侧一致）
      const files = []
      const walk = async (dir) => {
        let entries
        try {
          entries = await fs.readdir(dir, { withFileTypes: true })
        } catch {
          return
        }
        for (const e of entries) {
          const full = path.join(dir, e.name)
          if (e.isDirectory()) await walk(full)
          else if (e.name.toLowerCase().endsWith('.lrc')) files.push(full)
        }
      }
      await walk(input)
      files.sort()

      // 摊平成「行」，键为 路径#序号，便于逐行配对
      const out = []
      for (const file of files) {
        const text = await fs.readFile(file, 'utf8')
        const parsed = parseLrc(text)
        parsed.forEach((line, i) => {
          out.push({ key: file + '#' + i, start: line.start, end: line.end, text: line.text })
        })
      }
      return out
    },

    async runRust(input, native) {
      const files = JSON.parse(native.scanLyrics(input))
      const out = []
      for (const f of files) {
        f.lines.forEach((line, i) => {
          out.push({ key: f.path + '#' + i, start: line.start, end: line.end, text: line.text })
        })
      }
      return out
    },
  },
  // ───────────────────────── settings_io（R2-3） ─────────────────────────
  {
    id: 'settings_io',
    label: '设置读写（序列化层）',
    unit: '项',
    // 能力位：有写路径。写只落沙箱临时文件，真实文件由 guardFiles 防篡改校验兜底。
    // R2-4 的接入开关按此位控制激活范围。
    capabilities: { read: true, write: true },

    /**
     * 需要防篡改校验的真实文件。框架跑前后各算一次 SHA-256，必须一致。
     * 声明了 write 能力却不给 guardFiles 的模块会被框架直接拒跑。
     */
    get guardFiles() {
      return [REAL_SETTINGS, REAL_DB].filter(Boolean)
    },

    keyField: 'key',
    fields: ['value'],
    describe: (it) => it?.key ?? '(未知项)',

    /**
     * Node 侧：读真实设置 + 回环测试。
     *
     * 两类用例合成一个扁平结果集：
     *   read:<点分路径>       读路径 —— 真实文件解析后的每个叶子值
     *   loop:<用例>:<路径>    回环 —— 改一个键后写沙箱、读回，比对全量语义
     *
     * 真实文件只读；所有写操作落 os.tmpdir() 下的沙箱目录，用例跑完即删。
     */
    async runNode(input) {
      const fs = await import('node:fs/promises')
      const os = await import('node:os')
      const path = await import('node:path')

      const out = []
      if (!REAL_SETTINGS) return out

      // ── 读路径：真实文件照常读 ──
      const raw = await fs.readFile(REAL_SETTINGS, 'utf8')
      const root = JSON.parse(raw.charCodeAt(0) === 0xfeff ? raw.slice(1) : raw)

      // 摊平成叶子值，逐项比对（键序差异会体现为「只有一侧有」）
      const flatten = (v, prefix, sink) => {
        if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
          for (const k of Object.keys(v)) flatten(v[k], prefix ? prefix + '.' + k : k, sink)
        } else {
          // 数组与标量整体序列化，转义差异会直接暴露
          sink.push({ path: prefix, value: JSON.stringify(v) })
        }
      }
      const leaves = []
      flatten(root, '', leaves)
      for (const l of leaves) out.push({ key: 'read:' + l.path, value: l.value })

      // ── 回环路径：读 → 改一个键 → 序列化 → 读回 ──
      const sandbox = await fs.mkdtemp(path.join(os.tmpdir(), 'aurora-shadow-node-'))
      try {
        for (const c of LOOP_CASES) {
          const target = path.join(sandbox, c.id + '.json')
          const copy = JSON.parse(JSON.stringify(root))
          setByPath(copy, c.keyPath, c.value)
          // 原子写：临时文件 + rename，对齐 settings.ts 的 persist
          const tmp = target + '.tmp'
          await fs.writeFile(tmp, JSON.stringify(copy, null, 2), 'utf8')
          await fs.rename(tmp, target)
          // 读回后整体摊平，任何键序/转义/结构差异都会逐项暴露
          const back = JSON.parse(await fs.readFile(target, 'utf8'))
          const backLeaves = []
          flatten(back, '', backLeaves)
          for (const l of backLeaves) {
            out.push({ key: 'loop:' + c.id + ':' + l.path, value: l.value })
          }
        }
      } finally {
        // 用例跑完即删，不留垃圾
        await fs.rm(sandbox, { recursive: true, force: true })
      }
      return out
    },

    async runRust(input, native) {
      const fs = await import('node:fs/promises')
      const os = await import('node:os')
      const path = await import('node:path')

      const out = []
      if (!REAL_SETTINGS) return out

      const flatten = (v, prefix, sink) => {
        if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
          for (const k of Object.keys(v)) flatten(v[k], prefix ? prefix + '.' + k : k, sink)
        } else {
          sink.push({ path: prefix, value: JSON.stringify(v) })
        }
      }

      // ── 读路径 ──
      const root = JSON.parse(native.readSettings(REAL_SETTINGS))
      const leaves = []
      flatten(root, '', leaves)
      for (const l of leaves) out.push({ key: 'read:' + l.path, value: l.value })

      // ── 回环路径 ──
      const sandbox = await fs.mkdtemp(path.join(os.tmpdir(), 'aurora-shadow-rust-'))
      try {
        for (const c of LOOP_CASES) {
          const target = path.join(sandbox, c.id + '.json')
          // Rust 侧全程只读 REAL_SETTINGS，写只落 target（沙箱）
          const back = JSON.parse(
            native.settingsRoundtrip(REAL_SETTINGS, target, c.keyPath, JSON.stringify(c.value))
          )
          const backLeaves = []
          flatten(back, '', backLeaves)
          for (const l of backLeaves) {
            out.push({ key: 'loop:' + c.id + ':' + l.path, value: l.value })
          }
        }
      } finally {
        await fs.rm(sandbox, { recursive: true, force: true })
      }
      return out
    },

    /**
     * 数值按语义等值判定，格式差异单列报告（已与用户确认的判据）。
     *
     * 理由：Rust 的 f64→文本规则与 V8 的 JSON.stringify 不完全相同
     * （尾随零、指数阈值、最短往返表示）。settings.json 有 71 个非整数浮点，
     * 若按字节判，会被无害的写法差异淹没，真正的精度丢失反而看不见。
     * 这里对数值做 f64 位级比较，非数值仍要求完全相等。
     * 格式差异不判 FAIL，但由框架统计后单列，供人工审查。
     */
    compare(field, a, b) {
      if (a === b) return true
      if (field !== 'value') return false
      // 两侧都是合法 JSON 数字才走等值判定
      const na = Number(a)
      const nb = Number(b)
      if (!Number.isFinite(na) || !Number.isFinite(nb)) return false
      if (String(a).trim() === '' || String(b).trim() === '') return false
      return na === nb
    },

    /** 格式差异登记：语义相等但文本不同的项，由框架单列统计 */
    formatDiff(field, a, b) {
      return field === 'value' && a !== b
    },
  },
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