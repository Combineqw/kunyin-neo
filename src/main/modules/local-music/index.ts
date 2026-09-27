/**
 * 本地歌曲导入（LX「添加本地歌曲」）。
 *
 * 弹系统文件选择框（多选音频文件）→ music-metadata 解析标签 → LocalMusicItem 入歌单。
 * - id 为文件绝对路径的 SHA-1 前 52 bit（确定性数值，重复导入被 INSERT OR IGNORE 去重）；
 * - 标签缺失时回退文件名解析（"艺术家 - 标题" 或纯标题）；
 * - 内嵌封面不入库（song_json 会进 SQLite，塞 base64 图会把库撑爆），封面留空。
 */
import { dialog } from 'electron'
import { readdir } from 'node:fs/promises'
import { setImmediate } from 'node:timers/promises'
import { basename, extname, join } from 'node:path'
import type { LocalMusicItem } from '../../../common/types/music'
import { getMainWindow } from '../../windows/main'
import { AUDIO_EXTENSIONS, localSongId, parseLocalSong } from './core'
import { nativeParseTrack, nativeScanDirectory } from '../../native/bridge'

export { AUDIO_EXTENSIONS, localSongId, parseFileName, parseLocalSong } from './core'

export type LocalScanResult = {
  items: LocalMusicItem[]
  skipped: number
}

export type LocalScanProgress = {
  phase: 'collecting' | 'scanning'
  done: number
  total: number
  skipped: number
  currentPath?: string
}

/** 弹多选文件框；取消返回 null */
export async function pickLocalSongs(): Promise<string[] | null> {
  const win = getMainWindow()
  const r = await dialog.showOpenDialog(win ?? undefined!, {
    title: '添加本地歌曲',
    properties: ['openFile', 'multiSelections'],
    filters: [{ name: '音频文件', extensions: AUDIO_EXTENSIONS }]
  })
  return r.canceled || !r.filePaths.length ? null : r.filePaths
}

export async function pickLocalDirectory(): Promise<string | null> {
  const win = getMainWindow()
  const r = await dialog.showOpenDialog(win ?? undefined!, {
    title: '扫描本地音乐文件夹',
    properties: ['openDirectory']
  })
  return r.canceled || !r.filePaths[0] ? null : r.filePaths[0]
}

export async function scanLocalSongs(directory: string): Promise<LocalScanResult> {
  const result = nativeScanDirectory(directory)
  if (result) {
    return {
      items: result.tracks.map((track) => ({
        type: 'local',
        id: localSongId(track.path),
        title: track.title || basename(track.path),
        artist: track.artist,
        album: track.album,
        cover: '',
        duration: track.duration,
        qualities: {},
        ...(track.replayGain ? { replayGain: track.replayGain } : {}),
        filePath: track.path
      })),
      skipped: result.skippedNonAudio + result.parseFailed + result.walkErrors
    }
  }

  // Development builds and older installs may not contain the optional native
  // binary. Keep the existing Node parser as a functional fallback; the Rust
  // path remains the default when it is available.
  const out: LocalMusicItem[] = []
  let skipped = 0
  const visit = async (dir: string): Promise<void> => {
    let entries
    try {
      entries = await readdir(dir, { withFileTypes: true })
    } catch {
      skipped++
      return
    }
    for (const entry of entries) {
      const path = join(dir, entry.name)
      if (entry.isDirectory()) {
        await visit(path)
        continue
      }
      if (
        !entry.isFile() ||
        !AUDIO_EXTENSIONS.includes(extname(entry.name).slice(1).toLowerCase())
      ) {
        if (entry.isFile()) skipped++
        continue
      }
      try {
        const track = nativeParseTrack(path)
        if (track) {
          out.push({
            type: 'local',
            id: localSongId(track.path),
            title: track.title || basename(track.path),
            artist: track.artist,
            album: track.album,
            cover: '',
            duration: track.duration,
            qualities: {},
            ...(track.replayGain ? { replayGain: track.replayGain } : {}),
            filePath: track.path
          })
        } else {
          out.push(await parseLocalSong(path))
        }
      } catch {
        // One unreadable file must not abort the directory import.
        skipped++
      }
    }
  }
  await visit(directory)
  return { items: out, skipped }
}

/**
 * Incremental directory scan used by the progress island. The whole-directory
 * native API is intentionally not used here; each native parse yields between
 * files so IPC cancel requests remain responsive.
 */
export async function scanLocalSongsProgressive(
  directory: string,
  signal: AbortSignal,
  onProgress: (progress: LocalScanProgress) => void
): Promise<LocalScanResult & { cancelled: boolean }> {
  const files: string[] = []
  let skipped = 0
  const visit = async (dir: string): Promise<boolean> => {
    if (signal.aborted) return false
    let entries
    try {
      entries = await readdir(dir, { withFileTypes: true })
    } catch {
      skipped++
      return true
    }
    for (const entry of entries) {
      if (signal.aborted) return false
      const path = join(dir, entry.name)
      if (entry.isDirectory()) {
        if (!(await visit(path))) return false
      } else if (entry.isFile()) {
        if (AUDIO_EXTENSIONS.includes(extname(entry.name).slice(1).toLowerCase())) files.push(path)
        else skipped++
      }
      await setImmediate()
    }
    return true
  }

  if (!(await visit(directory))) return { items: [], skipped, cancelled: true }
  onProgress({ phase: 'scanning', done: 0, total: files.length, skipped })

  const items: LocalMusicItem[] = []
  for (let i = 0; i < files.length; i++) {
    if (signal.aborted) return { items: [], skipped, cancelled: true }
    const path = files[i]
    try {
      const track = nativeParseTrack(path)
      if (track) {
        items.push({
          type: 'local',
          id: localSongId(track.path),
          title: track.title || basename(track.path),
          artist: track.artist,
          album: track.album,
          cover: '',
          duration: track.duration,
          qualities: {},
          ...(track.replayGain ? { replayGain: track.replayGain } : {}),
          filePath: track.path
        })
      } else {
        items.push(await parseLocalSong(path))
      }
    } catch {
      skipped++
    }
    onProgress({ phase: 'scanning', done: i + 1, total: files.length, skipped, currentPath: path })
    await setImmediate()
  }
  return { items, skipped, cancelled: false }
}
