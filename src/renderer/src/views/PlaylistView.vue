<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import DetailHeader from '../components/DetailHeader.vue'
import SongRow from '../components/SongRow.vue'
import { usePlayerStore, type QueueSource } from '../stores/player'
import { useLibraryStore } from '../stores/library'
import { getMusicItemKey, type MusicItem, type MusicSource } from '@common'

const route = useRoute()
const player = usePlayerStore()
const library = useLibraryStore()

const info = ref<{ name: string; cover?: string; creator?: string; total?: number }>({ name: '' })
const tracks = ref<MusicItem[]>([])
const loading = ref(false)
const loadingMore = ref(false)
const hasMore = ref(false)
const loadMoreTarget = ref<HTMLElement>()
let loadSerial = 0
let onlinePage = 0
let loadMoreObserver: IntersectionObserver | undefined

async function loadOnlinePage(
  source: MusicSource,
  id: string,
  page: number
): Promise<{ result: MusicItem[]; hasNext: boolean }> {
  const response = await window.api.discover.playlistSongs(source, id, page, 100)
  return { result: response.result, hasNext: response.hasNext && response.result.length > 0 }
}

function observeLoadMoreTarget(): void {
  loadMoreObserver?.disconnect()
  const target = loadMoreTarget.value
  if (!target || !hasMore.value) return
  loadMoreObserver = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) void loadMore()
    },
    { rootMargin: '320px 0px' }
  )
  loadMoreObserver.observe(target)
}

async function loadMore(): Promise<void> {
  const source = route.query.source as MusicSource | undefined
  if (!source || loading.value || loadingMore.value || !hasMore.value) return
  const serial = loadSerial
  const id = String(route.params.playlistId)
  loadingMore.value = true
  const page = onlinePage + 1
  try {
    const response = await loadOnlinePage(source, id, page)
    if (serial !== loadSerial) return
    const known = new Set(tracks.value.map((item) => getMusicItemKey(item)))
    const appended = response.result.filter((item) => {
      const key = getMusicItemKey(item)
      if (known.has(key)) return false
      known.add(key)
      return true
    })
    tracks.value = appended.length ? [...tracks.value, ...appended] : tracks.value
    onlinePage = page
    hasMore.value = response.hasNext
  } finally {
    if (serial === loadSerial) {
      loadingMore.value = false
      await nextTick()
      observeLoadMoreTarget()
    }
  }
}

async function load(): Promise<void> {
  const serial = ++loadSerial
  loading.value = true
  loadingMore.value = false
  hasMore.value = false
  onlinePage = 0
  loadMoreObserver?.disconnect()
  tracks.value = []
  const source = route.query.source as MusicSource | undefined
  const id = String(route.params.playlistId)
  try {
    if (source) {
      // 在线歌单（来自搜索/详情跳转）
      const detail = await window.api.discover.playlistInfo(source, id)
      info.value = detail
        ? { name: detail.name, cover: detail.cover, creator: detail.creator, total: detail.total }
        : { name: '歌单' }
      const result = await loadOnlinePage(source, id, 0)
      if (serial === loadSerial) {
        tracks.value = result.result
        hasMore.value = result.hasNext
      }
    } else {
      // 本地歌单
      const pid = Number(id)
      const pl = library.playlists.find((p) => p.id === pid)
      info.value = { name: pl?.name ?? '歌单', cover: pl?.coverUrl, total: pl?.songCount }
      const result = await library.playlistSongs(pid)
      if (serial === loadSerial) tracks.value = result
    }
  } finally {
    if (serial === loadSerial) {
      loading.value = false
      await nextTick()
      observeLoadMoreTarget()
    }
  }
}

watch(() => [route.params.playlistId, route.query.source], load, { immediate: true })
onBeforeUnmount(() => loadMoreObserver?.disconnect())

function isActive(item: MusicItem): boolean {
  return !!player.current && getMusicItemKey(player.current) === getMusicItemKey(item)
}
// 歌单播放队列 = 整个歌单（上一首/下一首在单内导航）；在线/本地歌单均不累积进试听列表
function playSource(): QueueSource | undefined {
  const id = String(route.params.playlistId)
  const source = route.query.source as MusicSource | undefined
  return source
    ? { kind: 'platform', id: `${source}:${id}`, name: info.value.name }
    : { kind: 'local', id, name: info.value.name }
}
function play(item: MusicItem): void {
  player.playItem(item, tracks.value, { source: playSource() })
}
function playAll(): void {
  if (tracks.value.length) play(tracks.value[0])
}
</script>

<template>
  <div class="page">
    <DetailHeader
      :cover="info.cover"
      :title="info.name"
      :subtitle="info.creator"
      :meta="`${info.total ?? tracks.length} 首歌曲`"
      @play-all="playAll"
    />
    <div v-if="loading" class="hint">加载中…</div>
    <div v-else class="list">
      <SongRow
        v-for="(t, i) in tracks"
        :key="getMusicItemKey(t)"
        :item="t"
        :index="i"
        :active="isActive(t)"
        @play="play(t)"
      />
      <div ref="loadMoreTarget" class="load-more" aria-live="polite">
        <span v-if="loadingMore">加载更多…</span>
        <span v-else-if="hasMore">继续滚动以加载更多</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  width: 100%;
  min-height: 100%;
  box-sizing: border-box;
  padding-bottom: 16px;
}
.list {
  display: flex;
  flex-direction: column;
}
.list > :deep(.song-row) {
  content-visibility: auto;
  contain-intrinsic-size: 60px;
}
.load-more {
  min-height: 30px;
  padding: 10px 0;
  text-align: center;
  color: var(--color-font-label);
  font-size: 12px;
}
.hint {
  padding: 20px 0;
  text-align: center;
  font-size: 13px;
  color: var(--color-font-label);
}
</style>
