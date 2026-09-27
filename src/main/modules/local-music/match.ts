import type { MusicItem } from '../../../common/types/music'

export interface MatchTags {
  title: string
  artist: string
}

/** 用于跨来源匹配；文件名只用于展示，不会进入此函数。 */
export function normalizeMusicText(value: string): string {
  return value
    .normalize('NFKC')
    .toLocaleLowerCase()
    .replace(/\b(?:feat|ft)\.?\b/gu, '')
    .replace(/[^\p{L}\p{N}]+/gu, '')
}

function artistsMatch(left: string, right: string): boolean {
  const a = normalizeMusicText(left)
  const b = normalizeMusicText(right)
  return !!a && !!b && (a === b || a.includes(b) || b.includes(a))
}

/** 只接受标题、歌手和时长都能对上的唯一候选，歧义时返回 null。 */
export function selectEnrichmentCandidate(
  tags: MatchTags,
  duration: number,
  candidates: MusicItem[]
): MusicItem | null {
  const title = normalizeMusicText(tags.title)
  if (!title || !normalizeMusicText(tags.artist)) return null
  const matches = candidates.filter((candidate) => {
    if (candidate.type === 'local' || normalizeMusicText(candidate.title) !== title) return false
    if (!artistsMatch(tags.artist, candidate.artist)) return false
    if (duration > 0 && candidate.duration > 0 && Math.abs(duration - candidate.duration) > 5000)
      return false
    return true
  })
  return matches.length === 1 ? matches[0] : null
}
