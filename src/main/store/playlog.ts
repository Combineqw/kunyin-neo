/**
 * 本地播放事件持久化。
 * 该模块只把播放完成度、跳过行为和歌曲元数据写入 SQLite，推荐排序保持在 common/domain 的纯函数中。
 */
import type { MusicItem } from '@common'
import { getDb, TABLE_PLAY_EVENTS, TABLE_SONG_METADATA, TABLE_SONGS } from './db'

export interface PlayEventInput {
  item: MusicItem
  playedAt: number
  durationMs: number
  playedMs: number
  ended: boolean
}

const MAX_EVENTS = 10000
const MIN_PLAYED_MS = 1000
const SKIP_COMPLETION_THRESHOLD = 0.9

function asNonNegativeInt(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0
}

/**
 * 记录一次有效播放事件，并同步歌曲元数据用于本地推荐。
 *
 * @param input 播放时长、歌曲与结束状态的快照。
 * @returns 无返回值；事件持久化至 SQLite。
 */
export function recordPlayEvent(input: PlayEventInput): void {
  const durationMs = asNonNegativeInt(input.durationMs)
  const playedMs = Math.min(asNonNegativeInt(input.playedMs), durationMs || Number.MAX_SAFE_INTEGER)
  if (!input.ended && playedMs < MIN_PLAYED_MS) return

  const completion = durationMs > 0 ? Math.min(1, playedMs / durationMs) : input.ended ? 1 : 0
  const skipped = !input.ended && completion < SKIP_COMPLETION_THRESHOLD ? 1 : 0
  const db = getDb()
  db.prepare(`INSERT OR REPLACE INTO ${TABLE_SONGS} (song_id, source, song_json) VALUES (?, ?, ?)`).run(
    input.item.id,
    input.item.type,
    JSON.stringify(input.item)
  )
  db.prepare(
    `INSERT INTO ${TABLE_SONG_METADATA} (song_id, source, tags_json, updated_at)
     VALUES (?, ?, ?, ?)
     ON CONFLICT(song_id, source) DO UPDATE SET tags_json = excluded.tags_json, updated_at = excluded.updated_at`
  ).run(input.item.id, input.item.type, JSON.stringify(input.item.tags ?? []), input.playedAt)
  db.prepare(
    `INSERT INTO ${TABLE_PLAY_EVENTS}
      (song_id, source, played_at, duration_ms, played_ms, completion, skipped)
     VALUES (?, ?, ?, ?, ?, ?, ?)`
  ).run(input.item.id, input.item.type, input.playedAt, durationMs, playedMs, completion, skipped)
  db.prepare(
    `DELETE FROM ${TABLE_PLAY_EVENTS} WHERE id <=
      (SELECT id FROM ${TABLE_PLAY_EVENTS} ORDER BY id DESC LIMIT 1 OFFSET ?)`
  ).run(MAX_EVENTS)
}

/**
 * 读取当前累计播放事件数。
 *
 * @returns SQLite 中的播放事件数量。
 */
export function countPlayEvents(): number {
  const result = getDb().prepare(`SELECT COUNT(*) AS count FROM ${TABLE_PLAY_EVENTS}`).get() as { count: number }
  return result.count
}
