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
import { basename, extname, join } from 'node:path'
import type { LocalMusicItem } from '../../../common/types/music'
import { getMainWindow } from '../../windows/main'
import { AUDIO_EXTENSIONS, localSongId, parseLocalSong } from './core'
import { nativeScanDirectory } from '../../native/bridge'

export { AUDIO_EXTENSIONS, localSongId, parseFileName, parseLocalSong } from './core'

export type LocalScanResult = {
  items: LocalMusicItem[]
  skipped: number
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
      if (!entry.isFile() || !AUDIO_EXTENSIONS.includes(extname(entry.name).slice(1).toLowerCase())) {
        if (entry.isFile()) skipped++
        continue
      }
      try {
        out.push(await parseLocalSong(path))
      } catch {
        // One unreadable file must not abort the directory import.
        skipped++
      }
    }
  }
  await visit(directory)
  return { items: out, skipped }
}
