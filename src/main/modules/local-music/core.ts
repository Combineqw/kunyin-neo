import { createHash } from 'node:crypto'
import { basename, extname } from 'node:path'
import type { LocalMusicItem } from '../../../common/types/music'

export const AUDIO_EXTENSIONS = ['mp3', 'flac', 'wav', 'm4a', 'aac', 'ogg', 'opus', 'wma', 'ape']

/** 文件绝对路径 → 确定性数值 id（SHA-1 前 13 个 hex ≈ 52 bit，Number 安全范围内） */
export function localSongId(filePath: string): number {
  const hex = createHash('sha1').update(filePath).digest('hex').slice(0, 13)
  return parseInt(hex, 16)
}

/** "艺术家 - 标题.ext" / "标题.ext" → { title, artist } */
export function parseFileName(filePath: string): { title: string; artist: string } {
  const base = basename(filePath, extname(filePath)).trim()
  const sep = base.indexOf(' - ')
  if (sep > 0) {
    return { artist: base.slice(0, sep).trim(), title: base.slice(sep + 3).trim() }
  }
  return { title: base, artist: '' }
}

/** 解析单个文件为 LocalMusicItem；标签解析失败也不丢文件（回退文件名） */
export async function parseLocalSong(filePath: string): Promise<LocalMusicItem> {
  const fallback = parseFileName(filePath)
  let title = fallback.title
  let artist = fallback.artist
  let album = ''
  let duration = 0
  try {
    // music-metadata 为 ESM-only 包，主进程 CJS 侧用动态 import 加载
    const mm = await import('music-metadata')
    const meta = await mm.parseFile(filePath, { duration: true, skipCovers: true })
    if (meta.common.title?.trim()) title = meta.common.title.trim()
    const artists = meta.common.artists?.length ? meta.common.artists : [meta.common.artist ?? '']
    const joined = artists.filter(Boolean).join('、')
    if (joined) artist = joined
    album = meta.common.album?.trim() ?? ''
    duration = Math.round((meta.format.duration ?? 0) * 1000)
  } catch {
    /* 无标签/不支持的容器：回退文件名 + 时长 0（播放时 <audio> 会读到真实时长） */
  }
  return {
    type: 'local',
    id: localSongId(filePath),
    title,
    artist,
    album,
    cover: '',
    duration,
    qualities: {},
    filePath
  }
}
