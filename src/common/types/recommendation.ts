import type { MusicItem, MusicSource } from './music'

export interface RecommendationMetadata {
  tags: string[]
  genre?: string
  bpm?: number
}

export interface RecommendationStats {
  playCount: number
  completionCount: number
  skipCount: number
  completionRate: number
  lastPlayedAt?: number
}

export interface RecommendationSong extends RecommendationMetadata, RecommendationStats {
  item: MusicItem
  source: MusicSource
  songId: number
  isFavorite: boolean
}

export interface DailyRecommendationOptions {
  limit?: number
  maxPerArtist?: number
  maxPerGenre?: number
  recentSongKeys?: readonly string[]
  random?: () => number
}

export interface HeartbeatRecommendationOptions {
  limit?: number
  familiarRatio?: number
  random?: () => number
}

export interface DailyRecommendationEntry {
  song: RecommendationSong
  score: number
  reason: 'favorite' | 'completion' | 'cold-start'
}

export interface HeartbeatRecommendationEntry {
  song: RecommendationSong
  similarity: number
  familiar: boolean
}

export function getRecommendationSongKey(song: Pick<RecommendationSong, 'source' | 'songId'>): string {
  return `${song.source}_${song.songId}`
}

export function recommendationMetadataFromItem(item: MusicItem): RecommendationMetadata {
  return { tags: normalizeTags(item.tags) }
}

export function normalizeTags(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return [...new Set(value.filter((item): item is string => typeof item === 'string').map((item) => item.trim()).filter(Boolean))]
}
