import { existsSync, readFileSync } from 'node:fs'
import { basename, dirname, extname, join } from 'node:path'
import type { Lyric, MusicItem, MusicSource, LocalMusicItem } from '@common'
import { EMPTY_LYRIC } from '@common'
import { getProvider } from '../../providers'
import { setCachedLyric } from '../../cache/lyricCache'
import { selectEnrichmentCandidate } from './match'

const MATCH_SOURCES: MusicSource[] = ['wy', 'kw', 'kg', 'qq']

export interface EmbeddedLocalTags {
  title: string
  artist: string
  album: string
  lyrics: string
}

export interface LocalEnrichmentProgress {
  done: number
  total: number
  updated: number
  skipped: number
  currentPath?: string
}

export interface LocalEnrichmentResult {
  updated: number
  skipped: number
  cancelled: boolean
}

export interface LocalEnrichmentDeps {
  readTags: (filePath: string) => Promise<EmbeddedLocalTags | null>
  search: (source: MusicSource, keyword: string) => Promise<MusicItem[]>
  getLyric: (item: MusicItem) => Promise<Lyric>
}

function lyricHasContent(lyric: Lyric): boolean {
  return !!(lyric.char.trim() || lyric.lrc.trim())
}

function sidecarPath(filePath: string): string {
  return join(dirname(filePath), `${basename(filePath, extname(filePath))}.lrc`)
}

function readSidecarLyrics(filePath: string): string {
  const path = sidecarPath(filePath)
  if (!existsSync(path)) return ''
  try {
    const data = readFileSync(path)
    const utf8 = data.toString('utf8')
    return utf8.includes('�') ? new TextDecoder('gb18030').decode(data) : utf8
  } catch {
    return ''
  }
}

async function readEmbeddedTags(filePath: string): Promise<EmbeddedLocalTags | null> {
  try {
    const mm = await import('music-metadata')
    const meta = await mm.parseFile(filePath, { duration: false, skipCovers: true })
    const title = meta.common.title?.trim() ?? ''
    const artists = meta.common.artists?.filter(Boolean) ?? []
    const artist = (artists.length ? artists.join('、') : (meta.common.artist ?? '')).trim()
    // F1 明确认标签不认文件名；没有嵌入标题和艺术家就不联网猜测。
    if (!title || !artist) return null
    const lyrics = Array.isArray(meta.common.lyrics)
      ? meta.common.lyrics
          .map((entry) =>
            typeof entry === 'string'
              ? entry
              : typeof entry?.plainLyrics === 'string'
                ? entry.plainLyrics
                : ''
          )
          .filter(Boolean)
          .join('\n')
          .trim()
      : ''
    return { title, artist, album: meta.common.album?.trim() ?? '', lyrics }
  } catch {
    return null
  }
}

const defaultDeps: LocalEnrichmentDeps = {
  readTags: readEmbeddedTags,
  search: async (source, keyword) =>
    (await getProvider(source)?.search(keyword, 0, 10))?.result ?? [],
  getLyric: async (item) => (await getProvider(item.type)?.getLyric(item)) ?? { ...EMPTY_LYRIC }
}

async function findCandidate(
  tags: EmbeddedLocalTags,
  duration: number,
  deps: LocalEnrichmentDeps,
  signal: AbortSignal
): Promise<MusicItem | null> {
  const keyword = `${tags.title} ${tags.artist}`.trim()
  for (const source of MATCH_SOURCES) {
    if (signal.aborted) return null
    const candidates = await deps.search(source, keyword).catch(() => [])
    const match = selectEnrichmentCandidate(tags, duration, candidates)
    if (match) return match
  }
  return null
}

export async function completeLocalSong(
  item: LocalMusicItem,
  signal: AbortSignal,
  deps: LocalEnrichmentDeps = defaultDeps
): Promise<{ item: LocalMusicItem; updated: boolean; skipped: boolean }> {
  const tags = await deps.readTags(item.filePath)
  if (!tags || signal.aborted) return { item, updated: false, skipped: true }

  let next: LocalMusicItem = { ...item }
  let updated = false
  if (!next.album && tags.album) {
    next = { ...next, album: tags.album }
    updated = true
  }

  const localLyric = tags.lyrics || readSidecarLyrics(item.filePath)
  if (localLyric) setCachedLyric(item, { ...EMPTY_LYRIC, lrc: localLyric })

  const needsRemote = !next.cover || !next.album || !localLyric
  if (needsRemote) {
    const candidate = await findCandidate(tags, item.duration, deps, signal)
    if (candidate && !signal.aborted) {
      if (!next.cover && candidate.cover) {
        next = { ...next, cover: candidate.cover }
        updated = true
      }
      if (!next.album && candidate.album) {
        next = { ...next, album: candidate.album }
        updated = true
      }
      if (!localLyric) {
        const lyric = await deps.getLyric(candidate).catch(() => ({ ...EMPTY_LYRIC }))
        if (lyricHasContent(lyric)) updated = true
        // Also store an empty result as a negative cache so offline playback does not retry forever.
        setCachedLyric(item, lyric)
      }
    }
  }
  if (!localLyric) setCachedLyric(item, { ...EMPTY_LYRIC })
  return { item: next, updated, skipped: !updated && !localLyric }
}

export async function enrichLocalSongs(
  items: LocalMusicItem[],
  signal: AbortSignal,
  onProgress: (progress: LocalEnrichmentProgress) => void,
  onUpdate: (item: LocalMusicItem) => void,
  deps: LocalEnrichmentDeps = defaultDeps
): Promise<LocalEnrichmentResult> {
  const total = items.length
  let cursor = 0
  let done = 0
  let updated = 0
  let skipped = 0
  const worker = async (): Promise<void> => {
    while (!signal.aborted) {
      const index = cursor++
      if (index >= items.length) return
      const item = items[index]
      try {
        const result = await completeLocalSong(item, signal, deps)
        if (signal.aborted) return
        if (result.updated) {
          updated++
          onUpdate(result.item)
        }
        if (result.skipped) skipped++
      } catch {
        skipped++
      }
      done++
      onProgress({ done, total, updated, skipped, currentPath: item.filePath })
    }
  }
  await Promise.all(Array.from({ length: Math.min(4, Math.max(1, items.length)) }, () => worker()))
  return { updated, skipped, cancelled: signal.aborted }
}
