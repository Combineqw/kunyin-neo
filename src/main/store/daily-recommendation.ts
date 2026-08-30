import type { DailyRecommendationEntry } from '@common'
import { createDailyRecommendations, getRecommendationSongKey } from '@common'
import { getSettings } from './settings'
import { getDb, TABLE_RECOMMENDATION_CACHE } from './db'
import { queryRecommendationSongs } from './recommendation'

const CACHE_TTL_MS = 24 * 60 * 60 * 1000
const HISTORY_WINDOW_MS = 7 * 24 * 60 * 60 * 1000
const HISTORY_LIMIT = 210

interface DailyCache {
  generatedAt: number
  entries: DailyRecommendationEntry[]
}

interface RecommendationHistoryEntry {
  key: string
  generatedAt: number
}

let cache: DailyCache | null = null
let history: RecommendationHistoryEntry[] = []
const DAILY_KEY = 'daily-v1'
const HISTORY_KEY = 'daily-history-v1'

function readCache<T>(key: string): T | null {
  const row = getDb().prepare(`SELECT payload_json FROM ${TABLE_RECOMMENDATION_CACHE} WHERE cache_key = ?`).get(key) as { payload_json: string } | undefined
  try {
    return row ? (JSON.parse(row.payload_json) as T) : null
  } catch {
    return null
  }
}

function writeCache(key: string, payload: unknown): void {
  getDb().prepare(
    `INSERT INTO ${TABLE_RECOMMENDATION_CACHE} (cache_key, payload_json, updated_at) VALUES (?, ?, ?)
     ON CONFLICT(cache_key) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at`
  ).run(key, JSON.stringify(payload), Date.now())
}

function isFresh(value: DailyCache | null): value is DailyCache {
  return !!value && Date.now() - value.generatedAt < CACHE_TTL_MS
}

export function getDailyRecommendations(force = false): DailyCache {
  const settings = getSettings().player
  if (!settings.dailyRecommendationEnabled) {
    cache = { generatedAt: Date.now(), entries: [] }
    return cache
  }
  if (!cache) cache = readCache<DailyCache>(DAILY_KEY)
  if (!history.length) {
    const stored = readCache<RecommendationHistoryEntry[] | string[]>(HISTORY_KEY) ?? []
    history = stored.map((entry) =>
      typeof entry === 'string' ? { key: entry, generatedAt: Date.now() } : entry
    )
  }
  history = history.filter((entry) => Date.now() - entry.generatedAt < HISTORY_WINDOW_MS)
  if (!force && isFresh(cache)) return cache
  const songs = queryRecommendationSongs()
  const entries = createDailyRecommendations(songs, {
    limit: 30,
    maxPerArtist: settings.recommendationMaxPerArtist,
    recentSongKeys: history.map((entry) => entry.key)
  })
  cache = { generatedAt: Date.now(), entries }
  history = [
    ...entries.map((entry) => ({ key: getRecommendationSongKey(entry.song), generatedAt: cache!.generatedAt })),
    ...history
  ].slice(0, HISTORY_LIMIT)
  writeCache(DAILY_KEY, cache)
  writeCache(HISTORY_KEY, history)
  return cache
}

export function invalidateDailyRecommendations(): void {
  cache = null
}
