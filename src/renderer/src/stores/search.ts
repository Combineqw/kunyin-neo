/**
 * 在线搜索 Pinia store。
 * 管理音源、分页结果、热搜和本地搜索历史；不同搜索实体共用同一请求时序与分页状态。
 */
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type {
  AlbumInfoResult,
  ArtistInfoResult,
  MusicItem,
  MusicSource,
  PlayListInfoResult
} from '@common'
import { PLATFORMS } from '@common'
import { runSourceSearches } from '@common'

/** 搜索类型（对应 Android SearchType；joox 等只支持单曲，UI 按平台隐藏其余 Tab） */
export type SearchType = 'song' | 'playlist' | 'album' | 'artist'

export const SEARCH_TYPES: readonly { id: SearchType; label: string }[] = [
  { id: 'song', label: '单曲' },
  { id: 'playlist', label: '歌单' },
  { id: 'album', label: '专辑' },
  { id: 'artist', label: '歌手' }
] as const

/** 各平台支持的搜索类型；仅对已有真实 Provider 实现的实体开放 Tab。 */
export function supportedSearchTypes(source: MusicSource): SearchType[] {
  if (source === 'joox' || source === 'qqc') return ['song']
  if (source === 'wy' || source === 'qq') return ['song', 'playlist', 'album', 'artist']
  return ['song', 'album', 'artist']
}

const HISTORY_KEY = 'kunyin:searchHistory'
const HISTORY_MAX = 20
const PAGE_SIZE = 20

function loadHistory(): string[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY)
    return raw ? (JSON.parse(raw) as string[]) : []
  } catch {
    return []
  }
}

/**
 * 提供跨音源搜索、分页和本地历史记录的统一状态入口。
 *
 * @returns 搜索响应式状态与检索、分页、历史维护方法。
 */
export const useSearchStore = defineStore('search', () => {
  const source = ref<MusicSource>('wy')
  const aggregateMode = ref(false)
  const keyword = ref('')
  const searchType = ref<SearchType>('song')
  const results = ref<MusicItem[]>([])
  const playlistResults = ref<PlayListInfoResult[]>([])
  const albumResults = ref<AlbumInfoResult[]>([])
  const artistResults = ref<ArtistInfoResult[]>([])
  const loading = ref(false)
  const loadingMore = ref(false)
  const page = ref(0)
  const hasNext = ref(false)
  /** 当前音源热搜词（无检索时展示，仿 LX） */
  const hotWords = ref<string[]>([])
  const hotLoading = ref(false)
  /** 搜索历史（本地持久化，最近在前） */
  const history = ref<string[]>(loadHistory())
  const aggregateSources = ref<MusicSource[]>([])
  const aggregateResults = ref<Partial<Record<MusicSource, MusicItem[]>>>({})
  const aggregateStatus = ref<Partial<Record<MusicSource, 'loading' | 'done' | 'error'>>>({})
  const aggregateErrors = ref<Partial<Record<MusicSource, string>>>({})
  const aggregatePages = ref<Partial<Record<MusicSource, number>>>({})
  const aggregateHasNext = ref<Partial<Record<MusicSource, boolean>>>({})
  const aggregateLoadingMore = ref<Partial<Record<MusicSource, boolean>>>({})
  let requestGeneration = 0
  let hotGeneration = 0

  function persistHistory(): void {
    try {
      localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value))
    } catch {
      /* localStorage 不可用则忽略 */
    }
  }
  function addHistory(kw: string): void {
    const q = kw.trim()
    if (!q) return
    history.value = [q, ...history.value.filter((h) => h !== q)].slice(0, HISTORY_MAX)
    persistHistory()
  }
  function removeHistory(kw: string): void {
    history.value = history.value.filter((h) => h !== kw)
    persistHistory()
  }
  function clearHistory(): void {
    history.value = []
    persistHistory()
  }

  function clearResults(): void {
    results.value = []
    playlistResults.value = []
    albumResults.value = []
    artistResults.value = []
    page.value = 0
    hasNext.value = false
    aggregateSources.value = []
    aggregateResults.value = {}
    aggregateStatus.value = {}
    aggregateErrors.value = {}
    aggregatePages.value = {}
    aggregateHasNext.value = {}
    aggregateLoadingMore.value = {}
  }

  function invalidatePendingSearch(): void {
    requestGeneration++
    loading.value = false
    loadingMore.value = false
    aggregateLoadingMore.value = {}
    clearResults()
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error)
  }

  async function searchAllSources(generation: number, query: string): Promise<void> {
    aggregateStatus.value = Object.fromEntries(
      PLATFORMS.map((platform) => [platform, 'loading'])
    ) as Partial<Record<MusicSource, 'loading' | 'done' | 'error'>>
    await runSourceSearches(
      PLATFORMS,
      (platform) => window.api.search.songs(platform, query, 0, PAGE_SIZE),
      () => generation === requestGeneration,
      {
        resolved: (platform, response) => {
          aggregateResults.value[platform] = response.result
          aggregatePages.value[platform] = response.page
          aggregateHasNext.value[platform] = response.hasNext
          aggregateStatus.value[platform] = 'done'
        },
        rejected: (platform, error) => {
          aggregateStatus.value[platform] = 'error'
          aggregateErrors.value[platform] = errorMessage(error)
        },
        settled: (platform) => {
          if (!aggregateSources.value.includes(platform)) {
            aggregateSources.value = [...aggregateSources.value, platform]
          }
        }
      }
    )
  }

  async function search(reset = true): Promise<void> {
    if (!keyword.value.trim()) return
    const generation = reset ? ++requestGeneration : requestGeneration
    const query = keyword.value.trim()
    if (reset) addHistory(keyword.value)
    if (reset) {
      page.value = 0
      clearResults()
      loading.value = true
    } else {
      loadingMore.value = true
    }
    try {
      if (aggregateMode.value && searchType.value === 'song') {
        await searchAllSources(generation, query)
        return
      }
      switch (searchType.value) {
        case 'playlist': {
          const res = await window.api.discover.searchPlaylist(
            source.value,
            keyword.value,
            page.value,
            PAGE_SIZE
          )
          if (generation !== requestGeneration) return
          playlistResults.value = reset ? res.result : [...playlistResults.value, ...res.result]
          hasNext.value = res.hasNext
          break
        }
        case 'album': {
          const res = await window.api.discover.searchAlbum(
            source.value,
            keyword.value,
            page.value,
            PAGE_SIZE
          )
          if (generation !== requestGeneration) return
          albumResults.value = reset ? res.result : [...albumResults.value, ...res.result]
          hasNext.value = res.hasNext
          break
        }
        case 'artist': {
          const res = await window.api.discover.searchArtist(
            source.value,
            keyword.value,
            page.value,
            PAGE_SIZE
          )
          if (generation !== requestGeneration) return
          artistResults.value = reset ? res.result : [...artistResults.value, ...res.result]
          hasNext.value = res.hasNext
          break
        }
        default: {
          const res = await window.api.search.songs(
            source.value,
            keyword.value,
            page.value,
            PAGE_SIZE
          )
          if (generation !== requestGeneration) return
          results.value = reset ? res.result : [...results.value, ...res.result]
          hasNext.value = res.hasNext
        }
      }
    } finally {
      if (generation === requestGeneration) {
        loading.value = false
        loadingMore.value = false
      }
    }
  }

  async function loadMoreSource(platform: MusicSource): Promise<void> {
    if (
      !aggregateMode.value ||
      aggregateLoadingMore.value[platform] ||
      !aggregateHasNext.value[platform] ||
      aggregateStatus.value[platform] !== 'done'
    ) return
    const generation = requestGeneration
    const nextPage = (aggregatePages.value[platform] ?? 0) + 1
    aggregateLoadingMore.value[platform] = true
    delete aggregateErrors.value[platform]
    try {
      const response = await window.api.search.songs(platform, keyword.value.trim(), nextPage, PAGE_SIZE)
      if (generation !== requestGeneration) return
      aggregateResults.value[platform] = [
        ...(aggregateResults.value[platform] ?? []),
        ...response.result
      ]
      aggregatePages.value[platform] = response.page
      aggregateHasNext.value[platform] = response.hasNext
    } catch (error) {
      if (generation === requestGeneration) aggregateErrors.value[platform] = errorMessage(error)
    } finally {
      if (generation === requestGeneration) aggregateLoadingMore.value[platform] = false
    }
  }

  /** 下一页（追加） */
  async function loadMore(): Promise<void> {
    if (loading.value || loadingMore.value || !hasNext.value) return
    page.value += 1
    await search(false)
  }

  /** 切换搜索类型：清空结果并按现关键词重搜（对应 Android switchSearchType） */
  async function switchType(type: SearchType): Promise<void> {
    if (searchType.value === type) return
    searchType.value = type
    clearResults()
    if (keyword.value.trim()) await search()
  }

  /** 切平台时回退到该平台支持的类型（joox 只有单曲） */
  function ensureTypeSupported(): void {
    if (!supportedSearchTypes(source.value).includes(searchType.value)) {
      searchType.value = 'song'
    }
  }

  /** 搜索建议词（输入时下拉） */
  async function tips(kw: string): Promise<string[]> {
    if (!kw.trim()) return []
    try {
      return await window.api.search.tip(source.value, kw)
    } catch {
      return []
    }
  }

  /** 加载当前音源热搜词 */
  async function loadHot(): Promise<void> {
    const generation = ++hotGeneration
    hotLoading.value = true
    try {
      const words = await window.api.search.hot(source.value)
      if (generation === hotGeneration) hotWords.value = words
    } catch {
      if (generation === hotGeneration) hotWords.value = []
    } finally {
      if (generation === hotGeneration) hotLoading.value = false
    }
  }

  /** 用指定关键词检索（点击热搜词 / 外部触发） */
  async function searchFor(kw: string): Promise<void> {
    keyword.value = kw
    await search()
  }

  return {
    source,
    aggregateMode,
    keyword,
    searchType,
    results,
    playlistResults,
    albumResults,
    artistResults,
    loading,
    loadingMore,
    page,
    hasNext,
    hotWords,
    hotLoading,
    history,
    aggregateSources,
    aggregateResults,
    aggregateStatus,
    aggregateErrors,
    aggregatePages,
    aggregateHasNext,
    aggregateLoadingMore,
    search,
    loadMore,
    loadMoreSource,
    switchType,
    ensureTypeSupported,
    loadHot,
    searchFor,
    tips,
    addHistory,
    removeHistory,
    clearHistory,
    clearResults,
    invalidatePendingSearch
  }
})
