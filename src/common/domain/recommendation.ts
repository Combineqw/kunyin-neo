/**
 * 本地推荐领域的纯函数实现。
 * 只根据调用方提供的播放统计、元数据和随机源计算结果，不读取数据库也不持有应用状态。
 */
import type {
  DailyRecommendationEntry,
  DailyRecommendationOptions,
  HeartbeatRecommendationEntry,
  HeartbeatRecommendationOptions,
  RecommendationSong
} from '../types/recommendation'
import { getRecommendationSongKey } from '../types/recommendation'

const FAVORITE_WEIGHT = 12
const COMPLETION_WEIGHT = 5
const SKIP_WEIGHT = 4
const RECENT_PENALTY = 0.08
const EPSILON = 0.0001

function boundedRandom(random: () => number): number {
  return Math.max(0, Math.min(0.999999, random()))
}

function normalizedArtist(song: RecommendationSong): string {
  return song.item.artist.trim().toLocaleLowerCase()
}

function normalizedGenres(song: RecommendationSong): string[] {
  return [song.genre, ...song.tags]
    .filter((value): value is string => typeof value === 'string')
    .map((value) => value.trim().toLocaleLowerCase())
    .filter(Boolean)
}

function weightedPick<T>(entries: readonly { value: T; weight: number }[], random: () => number): T | null {
  const total = entries.reduce((sum, entry) => sum + Math.max(0, entry.weight), 0)
  if (total <= 0) return null
  let cursor = boundedRandom(random) * total
  for (const entry of entries) {
    cursor -= Math.max(0, entry.weight)
    if (cursor <= 0) return entry.value
  }
  return entries.at(-1)?.value ?? null
}

/**
 * 计算每日推荐的基础权重。
 * @param song 含本地统计和元数据的候选歌曲。
 * @param recentSongKeys 最近已推荐或播放的歌曲键集合。
 * @returns 用于加权抽样的非负分值。
 */
export function getDailyRecommendationScore(song: RecommendationSong, recentSongKeys: ReadonlySet<string>): number {
  const completionSignal = song.completionRate * COMPLETION_WEIGHT + Math.min(song.playCount, 20) * 0.15
  const favoriteSignal = song.isFavorite ? FAVORITE_WEIGHT : 0
  const skipSignal = Math.min(song.skipCount, 10) * SKIP_WEIGHT
  const base = Math.max(EPSILON, 1 + completionSignal + favoriteSignal - skipSignal)
  return recentSongKeys.has(getRecommendationSongKey(song)) ? base * RECENT_PENALTY : base
}

/**
 * 根据本地偏好和多样性上限生成每日推荐。
 * @param songs 候选歌曲池。
 * @param options 数量、歌手/流派上限及可注入随机源。
 * @returns 已附带分数与推荐理由的推荐条目。
 */
export function createDailyRecommendations(
  songs: readonly RecommendationSong[],
  options: DailyRecommendationOptions = {}
): DailyRecommendationEntry[] {
  const limit = Math.max(1, Math.round(options.limit ?? 30))
  const maxPerArtist = Math.max(1, Math.round(options.maxPerArtist ?? 2))
  const maxPerGenre = Math.max(1, Math.round(options.maxPerGenre ?? limit))
  const random = options.random ?? Math.random
  const recentSongKeys = new Set(options.recentSongKeys ?? [])
  const remaining = [...songs]
  const selected: DailyRecommendationEntry[] = []
  const artistCounts = new Map<string, number>()
  const genreCounts = new Map<string, number>()

  while (selected.length < limit && remaining.length) {
    const candidates = remaining.filter((song) => {
      const artist = normalizedArtist(song)
      if (artist && (artistCounts.get(artist) ?? 0) >= maxPerArtist) return false
      return normalizedGenres(song).every((genre) => (genreCounts.get(genre) ?? 0) < maxPerGenre)
    })
    if (!candidates.length) break
    const picked = weightedPick(
      candidates.map((song) => ({ value: song, weight: getDailyRecommendationScore(song, recentSongKeys) })),
      random
    )
    if (!picked) break
    const index = remaining.indexOf(picked)
    remaining.splice(index, 1)
    const score = getDailyRecommendationScore(picked, recentSongKeys)
    selected.push({
      song: picked,
      score,
      reason: picked.isFavorite ? 'favorite' : picked.playCount > 0 ? 'completion' : 'cold-start'
    })
    const artist = normalizedArtist(picked)
    if (artist) artistCounts.set(artist, (artistCounts.get(artist) ?? 0) + 1)
    for (const genre of normalizedGenres(picked)) genreCounts.set(genre, (genreCounts.get(genre) ?? 0) + 1)
  }
  return selected
}

/**
 * 计算两首歌的本地内容相似度。
 * @param seed 心动模式的种子歌曲。
 * @param candidate 待比较候选歌曲。
 * @returns 基于歌手、标签/流派和 BPM 的相似度分数。
 */
export function getSongSimilarity(seed: RecommendationSong, candidate: RecommendationSong): number {
  if (getRecommendationSongKey(seed) === getRecommendationSongKey(candidate)) return 0
  let score = 0
  if (normalizedArtist(seed) && normalizedArtist(seed) === normalizedArtist(candidate)) score += 10

  const seedGenres = new Set(normalizedGenres(seed))
  const sharedGenreCount = normalizedGenres(candidate).filter((genre) => seedGenres.has(genre)).length
  score += sharedGenreCount * 4

  if (seed.bpm != null && candidate.bpm != null) {
    const bpmDelta = Math.abs(seed.bpm - candidate.bpm)
    score += Math.max(0, 3 - bpmDelta / 12)
  }
  return score
}

function similarityWeight(entry: { song: RecommendationSong; similarity: number }): number {
  const familiarity = entry.song.isFavorite || entry.song.completionRate >= 0.65 ? 1.4 : 1
  return Math.max(EPSILON, entry.similarity * familiarity + EPSILON)
}

/**
 * 按熟悉歌曲与探索歌曲的比例创建心动模式队列。
 * @param seed 当前用于扩展的种子歌曲。
 * @param songs 候选歌曲池。
 * @param options 队列长度、熟悉比例及可注入随机源。
 * @returns 按抽样顺序输出的心动推荐条目。
 */
export function createHeartbeatRecommendations(
  seed: RecommendationSong,
  songs: readonly RecommendationSong[],
  options: HeartbeatRecommendationOptions = {}
): HeartbeatRecommendationEntry[] {
  const limit = Math.max(1, Math.round(options.limit ?? 30))
  const familiarRatio = Math.max(0, Math.min(1, options.familiarRatio ?? 0.7))
  const random = options.random ?? Math.random
  const entries = songs
    .map((song) => ({
      song,
      similarity: getSongSimilarity(seed, song),
      familiar: song.isFavorite || song.completionRate >= 0.65
    }))
    .filter((entry) => entry.similarity > 0)
  const familiarTarget = Math.round(limit * familiarRatio)
  const selected: HeartbeatRecommendationEntry[] = []

  const pickFrom = (pool: HeartbeatRecommendationEntry[]): void => {
    while (selected.length < limit && pool.length) {
      const picked = weightedPick(pool.map((entry) => ({ value: entry, weight: similarityWeight(entry) })), random)
      if (!picked) return
      selected.push(picked)
      pool.splice(pool.indexOf(picked), 1)
    }
  }

  const familiar = entries.filter((entry) => entry.familiar)
  const discovery = entries.filter((entry) => !entry.familiar)
  while (selected.length < familiarTarget && familiar.length) {
    const picked = weightedPick(familiar.map((entry) => ({ value: entry, weight: similarityWeight(entry) })), random)
    if (!picked) break
    selected.push(picked)
    familiar.splice(familiar.indexOf(picked), 1)
  }
  const discoveryTarget = Math.min(limit - selected.length, Math.max(0, limit - familiarTarget))
  while (selected.length < familiarTarget + discoveryTarget && discovery.length) {
    const picked = weightedPick(discovery.map((entry) => ({ value: entry, weight: similarityWeight(entry) })), random)
    if (!picked) break
    selected.push(picked)
    discovery.splice(discovery.indexOf(picked), 1)
  }
  pickFrom([...familiar, ...discovery])
  return selected
}
