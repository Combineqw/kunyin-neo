import type { MusicItem, RecommendationSong } from '@common'
import {
  getDb,
  SYSTEM_KIND_FAVORITES,
  TABLE_PLAY_EVENTS,
  TABLE_PLAYLIST_SONGS,
  TABLE_PLAYLISTS,
  TABLE_SONG_METADATA,
  TABLE_SONGS
} from './db'

interface RecommendationRow {
  song_id: number
  source: string
  song_json: string
  tags_json: string | null
  genre: string | null
  bpm: number | null
  play_count: number
  completion_count: number
  skip_count: number
  completion_rate: number
  last_played_at: number | null
  is_favorite: number
}

function parseSong(value: string): MusicItem | null {
  try {
    return JSON.parse(value) as MusicItem
  } catch {
    return null
  }
}

function parseTags(value: string | null): string[] {
  try {
    const parsed = value ? JSON.parse(value) : []
    return Array.isArray(parsed)
      ? parsed.filter((tag): tag is string => typeof tag === 'string').map((tag) => tag.trim()).filter(Boolean)
      : []
  } catch {
    return []
  }
}

/** 从本地歌曲快照、播放事件和 favorites 系统歌单聚合推荐候选。 */
export function queryRecommendationSongs(limit = 5000): RecommendationSong[] {
  const rows = getDb()
    .prepare(
      `SELECT s.song_id, s.source, s.song_json, m.tags_json, m.genre, m.bpm,
              COALESCE(stats.play_count, 0) AS play_count,
              COALESCE(stats.completion_count, 0) AS completion_count,
              COALESCE(stats.skip_count, 0) AS skip_count,
              COALESCE(stats.completion_rate, 0) AS completion_rate,
              stats.last_played_at,
              CASE WHEN fav.song_id IS NULL THEN 0 ELSE 1 END AS is_favorite
       FROM ${TABLE_SONGS} s
       LEFT JOIN ${TABLE_SONG_METADATA} m ON m.song_id = s.song_id AND m.source = s.source
       LEFT JOIN (
         SELECT song_id, source, COUNT(*) AS play_count,
                SUM(CASE WHEN completion >= 0.9 THEN 1 ELSE 0 END) AS completion_count,
                SUM(skipped) AS skip_count, AVG(completion) AS completion_rate,
                MAX(played_at) AS last_played_at
         FROM ${TABLE_PLAY_EVENTS}
         GROUP BY song_id, source
       ) stats ON stats.song_id = s.song_id AND stats.source = s.source
       LEFT JOIN (
         SELECT DISTINCT ps.song_id, ps.source
         FROM ${TABLE_PLAYLIST_SONGS} ps
         INNER JOIN ${TABLE_PLAYLISTS} p ON p.playlist_id = ps.playlist_id
         WHERE p.system_kind = ?
       ) fav ON fav.song_id = s.song_id AND fav.source = s.source
       ORDER BY COALESCE(stats.last_played_at, 0) DESC, s.rowid DESC
       LIMIT ?`
    )
    .all(SYSTEM_KIND_FAVORITES, Math.max(1, Math.min(5000, Math.round(limit)))) as RecommendationRow[]

  return rows.flatMap((row) => {
    const item = parseSong(row.song_json)
    if (!item) return []
    return [{
      item,
      source: item.type,
      songId: item.id,
      tags: parseTags(row.tags_json ?? JSON.stringify(item.tags ?? [])),
      genre: row.genre ?? undefined,
      bpm: row.bpm ?? undefined,
      playCount: row.play_count,
      completionCount: row.completion_count,
      skipCount: row.skip_count,
      completionRate: Math.max(0, Math.min(1, row.completion_rate)),
      lastPlayedAt: row.last_played_at ?? undefined,
      isFavorite: row.is_favorite !== 0
    } satisfies RecommendationSong]
  })
}
