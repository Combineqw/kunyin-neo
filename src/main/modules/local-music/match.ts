import type { MusicItem } from '../../../common/types/music'
import {
  nativeMatcherArtistsMatch,
  nativeMatcherNormalizeMusicText,
  nativeMatcherRankCandidates,
  nativeMatcherScoreMusicMatch,
  nativeMatcherSelectEnrichmentCandidate
} from '../../native/bridge-runtime.js'

export interface MatchTags {
  title: string
  artist: string
}

/** 用于跨来源匹配；文件名只用于展示，不会进入此函数。优先走 Rust 原生高性能正则与 Unicode 归一化。 */
export function normalizeMusicText(value: string): string {
  return nativeMatcherNormalizeMusicText(value)
}

export function artistsMatch(left: string, right: string): boolean {
  return nativeMatcherArtistsMatch(left, right)
}

/** 只接受标题、歌手和时长都能对上的唯一候选，歧义时返回 null。优先走 Rust 原生候选过滤。 */
export function selectEnrichmentCandidate(
  tags: MatchTags,
  duration: number,
  candidates: MusicItem[]
): MusicItem | null {
  return nativeMatcherSelectEnrichmentCandidate(tags, duration, candidates)
}

/** 计算本地曲目与在线候选的综合相似度匹配评分 (0.0 ..= 1.0)。 */
export function calculateMatchScore(
  tags: MatchTags,
  duration: number,
  candidate: MusicItem
): number {
  return nativeMatcherScoreMusicMatch(
    tags.title,
    tags.artist,
    duration,
    candidate.title,
    candidate.artist,
    candidate.duration
  )
}

/** 候选列表优选排序：按综合匹配得分从高到低排序。 */
export function rankEnrichmentCandidates(
  tags: MatchTags,
  duration: number,
  candidates: MusicItem[]
): Array<{ item: MusicItem; score: number }> {
  return nativeMatcherRankCandidates(tags, duration, candidates)
}
