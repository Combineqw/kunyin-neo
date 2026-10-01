<script setup lang="ts">
import { ref, watch } from 'vue'
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
let loadSerial = 0

/**
 * 在线歌单接口按页返回歌曲。详情页此前只取第一页；接口在歌曲详情补全失败、
 * 限流或返回稀疏结果时，首屏可能只有几行，既看起来像不能滚动，也漏掉后续歌曲。
 * 这里在进入详情时把分页结果合并，列表滚动仍由布局层的 .view 统一处理。
 */
async function loadOnlineTracks(source: MusicSource, id: string): Promise<MusicItem[]> {
  const all: MusicItem[] = []
  const seen = new Set<string>()

  // 100 页是防护上限，正常歌单只需数次请求；hasNext 由 provider 根据真实总数给出。
  for (let page = 0; page < 100; page++) {
    const res = await window.api.discover.playlistSongs(source, id, page, 100)
    for (const item of res.result) {
      const key = getMusicItemKey(item)
      if (seen.has(key)) continue
      seen.add(key)
      all.push(item)
    }
    // Providers should report hasNext accurately, but an empty page is always
    // a terminal condition and prevents a broken endpoint from causing 100
    // redundant requests while opening the detail page.
    if (!res.hasNext || !res.result.length) break
  }
  return all
}

async function load(): Promise<void> {
  const serial = ++loadSerial
  loading.value = true
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
      const result = await loadOnlineTracks(source, id)
      if (serial === loadSerial) tracks.value = result
    } else {
      // 本地歌单
      const pid = Number(id)
      const pl = library.playlists.find((p) => p.id === pid)
      info.value = { name: pl?.name ?? '歌单', cover: pl?.coverUrl, total: pl?.songCount }
      const result = await library.playlistSongs(pid)
      if (serial === loadSerial) tracks.value = result
    }
  } finally {
    if (serial === loadSerial) loading.value = false
  }
}

watch(() => [route.params.playlistId, route.query.source], load, { immediate: true })

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
.hint {
  padding: 20px 0;
  text-align: center;
  font-size: 13px;
  color: var(--color-font-label);
}
</style>
