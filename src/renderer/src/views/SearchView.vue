<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { useSearchStore, supportedSearchTypes, SEARCH_TYPES } from '../stores/search'
import { usePlayerStore } from '../stores/player'
import { useArtistStore } from '../stores/artist'
import {
  PLATFORMS,
  PLATFORM_NAMES,
  getMusicItemKey,
  type ArtistInfoResult,
  type MusicSource
} from '@common'
import SongRow from '../components/SongRow.vue'
import AppTabs from '../components/AppTabs.vue'
import AppIcon from '../components/AppIcon.vue'
import { coverUrl } from '../utils/cover'

defineOptions({ name: 'SearchView' })

const router = useRouter()
const searchStore = useSearchStore()
const player = usePlayerStore()
const artistStore = useArtistStore()
const {
  source,
  keyword,
  searchType,
  results,
  playlistResults,
  albumResults,
  artistResults,
  loading,
  loadingMore,
  hasNext,
  hotWords,
  history
} = storeToRefs(searchStore)

const sourceTabs = PLATFORMS.map((p) => ({ id: p, label: PLATFORM_NAMES[p] }))

// 搜索类型 Tab（对应 Android SearchScreen 的类型 chips；joox 只支持单曲时整排隐藏）
const typeTabs = computed(() =>
  SEARCH_TYPES.filter((t) => supportedSearchTypes(source.value).includes(t.id))
)
const showTypeTabs = computed(() => typeTabs.value.length > 1)
const STAGGER_LIMIT = 50

function staggerStyle(index: number): Record<string, string> {
  return { '--stagger-delay': `${index * 30}ms` }
}

function switchSource(id: string): void {
  source.value = id as MusicSource
  searchStore.ensureTypeSupported()
  if (keyword.value) void searchStore.search()
  else void searchStore.loadHot()
}

function isActive(item: (typeof results.value)[number]): boolean {
  return !!player.current && getMusicItemKey(player.current) === getMusicItemKey(item)
}

function openPlaylist(id: string): void {
  void router.push({ name: 'playlist', params: { playlistId: id }, query: { source: source.value } })
}
function openAlbum(id: string): void {
  void router.push({ name: 'album', params: { albumKey: `${source.value}:${id}` } })
}
/** 先缓存搜索结果里的歌手信息，详情页进入即可渲染头像/名称，不必空等接口 */
function openArtist(a: ArtistInfoResult): void {
  artistStore.cachePreview(a)
  void router.push({ name: 'artist', params: { artistKey: `${source.value}:${a.id}` } })
}

const showBlank = computed(
  () =>
    !loading.value &&
    !keyword.value &&
    !results.value.length &&
    !playlistResults.value.length &&
    !albumResults.value.length &&
    !artistResults.value.length
)

onMounted(() => {
  if (!results.value.length && !keyword.value) void searchStore.loadHot()
})

// 音源热搜词随 tab 切换刷新（无检索时）
watch(source, () => {
  if (!keyword.value) void searchStore.loadHot()
})
</script>

<template>
  <div class="search-view">
    <div class="header">
      <AppTabs :model-value="source" :list="sourceTabs" @change="switchSource" />
      <AppTabs
        v-if="showTypeTabs"
        class="type-tabs"
        :model-value="searchType"
        :list="typeTabs"
        @change="(id) => void searchStore.switchType(id as typeof searchType)"
      />
    </div>
    <div class="body scroll">
      <div v-if="loading" class="skeleton-list" aria-label="搜索中">
        <span v-for="i in 6" :key="i" class="skeleton-row">
          <i class="skeleton-cover" />
          <i class="skeleton-copy" />
        </span>
      </div>

      <!-- 单曲结果 -->
      <template v-else-if="searchType === 'song'">
        <TransitionGroup v-if="results.length" name="search-result" tag="div" class="list">
          <!-- 试听模型：该曲进试听列表，队列 = 整个试听列表（不变成搜索结果，也不单曲循环） -->
          <div
            v-for="(item, i) in results"
            :key="getMusicItemKey(item)"
            :class="{ 'stagger-item': i < STAGGER_LIMIT }"
            :style="i < STAGGER_LIMIT ? staggerStyle(i) : undefined"
          >
            <SongRow
              :item="item"
              :index="i"
              :active="isActive(item)"
              @play="player.playInTrial(item)"
            />
          </div>
        </TransitionGroup>
        <div v-else-if="keyword" class="empty">未找到「{{ keyword }}」相关歌曲</div>
      </template>

      <!-- LX 风格搜歌单：仅作为搜索类型存在，不占用独立侧栏页面。 -->
      <template v-else-if="searchType === 'playlist'">
        <TransitionGroup v-if="playlistResults.length" name="search-result" tag="div" class="card-list">
          <button
            v-for="(playlist, i) in playlistResults"
            :key="playlist.id"
            class="card-row"
            :class="{ 'stagger-item': i < STAGGER_LIMIT }"
            :style="i < STAGGER_LIMIT ? staggerStyle(i) : undefined"
            @click="openPlaylist(playlist.id)"
          >
            <img v-if="playlist.cover" class="cover" :src="coverUrl(playlist.cover)" loading="lazy" alt="" />
            <div v-else class="cover placeholder"><AppIcon name="library" :size="22" /></div>
            <div class="meta">
              <span class="name ellipsis">{{ playlist.name }}</span>
              <span class="sub ellipsis">
                {{
                  [
                    playlist.creator,
                    playlist.total ? `${playlist.total} 首` : '',
                    playlist.playCount ? `${playlist.playCount} 次播放` : ''
                  ]
                    .filter(Boolean)
                    .join(' · ')
                }}
              </span>
            </div>
          </button>
        </TransitionGroup>
        <div v-else-if="keyword" class="empty">未找到「{{ keyword }}」相关歌单</div>
      </template>

      <!-- 专辑结果（Android AlbumSearchResultsList：圆角封面 + 名称 + 歌手 · N首） -->
      <template v-else-if="searchType === 'album'">
        <TransitionGroup v-if="albumResults.length" name="search-result" tag="div" class="card-list">
          <button
            v-for="(a, i) in albumResults"
            :key="a.id"
            class="card-row"
            :class="{ 'stagger-item': i < STAGGER_LIMIT }"
            :style="i < STAGGER_LIMIT ? staggerStyle(i) : undefined"
            @click="openAlbum(a.id)"
          >
            <img v-if="a.cover" class="cover" :src="coverUrl(a.cover)" loading="lazy" alt="" />
            <div v-else class="cover placeholder"><AppIcon name="library" :size="22" /></div>
            <div class="meta">
              <span class="name ellipsis">{{ a.name }}</span>
              <span class="sub ellipsis">
                {{
                  [a.artist, a.total ? `${a.total} 首` : '', a.publishTime]
                    .filter(Boolean)
                    .join(' · ')
                }}
              </span>
            </div>
          </button>
        </TransitionGroup>
        <div v-else-if="keyword" class="empty">未找到「{{ keyword }}」相关专辑</div>
      </template>

      <!-- 歌手结果（Android ArtistSearchResultsList：圆形头像 + 名称 + N张专辑 · M首） -->
      <template v-else>
        <TransitionGroup v-if="artistResults.length" name="search-result" tag="div" class="card-list">
          <button
            v-for="(a, i) in artistResults"
            :key="a.id"
            class="card-row"
            :class="{ 'stagger-item': i < STAGGER_LIMIT }"
            :style="i < STAGGER_LIMIT ? staggerStyle(i) : undefined"
            @click="openArtist(a)"
          >
            <img
              v-if="a.cover"
              class="cover round"
              :src="coverUrl(a.cover)"
              loading="lazy"
              alt=""
            />
            <div v-else class="cover round placeholder"><AppIcon name="library" :size="22" /></div>
            <div class="meta">
              <span class="name ellipsis">{{ a.name }}</span>
              <span class="sub ellipsis">
                {{
                  [
                    a.albumCount ? `${a.albumCount} 张专辑` : '',
                    a.songCount ? `${a.songCount} 首` : ''
                  ]
                    .filter(Boolean)
                    .join(' · ')
                }}
              </span>
            </div>
          </button>
        </TransitionGroup>
        <div v-else-if="keyword" class="empty">未找到「{{ keyword }}」相关歌手</div>
      </template>

      <!-- 加载更多（Android 无限滚动的按钮版） -->
      <div v-if="!loading && hasNext && keyword" class="more">
        <button class="more-btn" :disabled="loadingMore" @click="void searchStore.loadMore()">
          {{ loadingMore ? '加载中…' : '加载更多' }}
        </button>
      </div>

      <button v-if="showBlank" class="recommend-card pressable" @click="void router.push({ name: 'recommendations' })">
        <span class="recommend-icon"><AppIcon name="daily-recommendation" :size="22" /></span>
        <span class="recommend-copy"><b>每日推荐与心动模式</b><small>从你的本地收听习惯出发，发现下一首想听的歌</small></span>
        <AppIcon name="chevron-right" :size="18" />
      </button>

      <!-- LX BlankView：热门搜索 / 搜索历史 chip 流 -->
      <div v-if="showBlank" class="blank">
        <dl v-if="hotWords.length" class="group">
          <dt class="group-title">热门搜索</dt>
          <dd class="group-body">
            <button
              v-for="(w, i) in hotWords"
              :key="w + i"
              class="chip ellipsis pressable"
              @click="searchStore.searchFor(w)"
            >
              {{ w }}
            </button>
          </dd>
        </dl>
        <dl v-if="history.length" class="group">
          <dt class="group-title">
            <span class="title">
              搜索历史
              <button class="clear" title="清空搜索历史" @click="searchStore.clearHistory()">
                <AppIcon name="trash" :size="15" />
              </button>
            </span>
          </dt>
          <dd class="group-body">
            <button
              v-for="h in history"
              :key="h"
              class="chip ellipsis pressable"
              @click="searchStore.searchFor(h)"
            >
              {{ h }}
            </button>
          </dd>
        </dl>
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-view {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.header {
  flex: none;
  padding-top: 10px;
}
.type-tabs {
  margin-top: 2px;
  padding-bottom: 2px;
  border-bottom: var(--color-list-header-border-bottom);
}
.body {
  flex: 1;
  min-height: 0;
  padding: 10px 15px 20px;
}
.list {
  display: flex;
  flex-direction: column;
}
.search-result-enter-active {
  animation: none;
}
.search-result-enter-active.stagger-item {
  animation: search-result-in var(--anim-dur-base) var(--anim-ease-standard) both;
  animation-delay: var(--stagger-delay, 0ms);
}
.search-result-move {
  transition: transform var(--anim-dur-fast) var(--anim-ease-smooth);
}
.search-result-leave-active {
  position: absolute;
  opacity: 0;
  transform: translate3d(-8px, 0, 0);
}
@keyframes search-result-in {
  from {
    opacity: 0;
    transform: translate3d(0, 12px, 0);
  }
  to {
    opacity: 1;
    transform: translate3d(0, 0, 0);
  }
}
.skeleton-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.skeleton-row {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 60px;
}
.skeleton-cover,
.skeleton-copy {
  position: relative;
  overflow: hidden;
  display: block;
  background-color: var(--color-primary-background);
}
.skeleton-cover {
  width: 42px;
  height: 42px;
  border-radius: 7px;
}
.skeleton-copy {
  width: min(48%, 300px);
  height: 13px;
  border-radius: 6px;
}
.skeleton-cover::after,
.skeleton-copy::after {
  content: '';
  position: absolute;
  inset: 0;
  background-color: var(--color-primary-background-hover);
  transform: translate3d(-115%, 0, 0) skewX(-18deg);
  animation: search-shimmer var(--anim-dur-slow) var(--anim-ease-smooth) infinite;
}
@keyframes search-shimmer {
  to {
    transform: translate3d(115%, 0, 0) skewX(-18deg);
  }
}
.recommend-card { display:flex; align-items:center; gap:12px; width:100%; margin:3px 0 22px; padding:15px; border:1px solid var(--color-border); border-radius:12px; color:var(--color-font); text-align:left; background:linear-gradient(120deg, var(--color-primary-background), var(--color-main-background)); cursor:pointer; transition:opacity var(--anim-dur-fast) var(--anim-ease-smooth); }
/* hover 上浮改喂 --press-lift，交给 .pressable 与按下缩放复合 */
.recommend-card:hover { opacity:0.88; --press-lift:-1px; }
.recommend-icon { display:grid; place-items:center; width:38px; height:38px; border-radius:10px; color:var(--color-primary); background:var(--color-primary-light-300-alpha-700); }
.recommend-copy { flex:1; display:grid; gap:4px; min-width:0; } .recommend-copy b { font-size:13px; } .recommend-copy small { color:var(--color-font-label); font-size:11px; }
.hint,
.empty {
  padding: 15px 0;
  color: var(--color-font-label);
  font-size: 13px;
}

/* 专辑/歌手结果行（对应 Android 搜索结果的 56dp 封面行） */
.card-list {
  display: flex;
  flex-direction: column;
}
.card-row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 7px 8px;
  border-radius: var(--radius-border);
  text-align: left;
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth), transform var(--anim-dur-fast) var(--anim-ease-standard);
}
.card-row:hover {
  background-color: var(--color-primary-background-hover);
}
.cover {
  flex: none;
  width: 52px;
  height: 52px;
  border-radius: 8px;
  object-fit: cover;
  background: var(--color-primary-background);
}
.cover.round {
  border-radius: 50%;
}
.cover.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-font-label);
}
.meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.name {
  font-size: 13.5px;
  color: var(--color-font);
}
.sub {
  font-size: 12px;
  color: var(--color-font-label);
}

.more {
  display: flex;
  justify-content: center;
  padding: 14px 0 4px;
}
.more-btn {
  padding: 7px 22px;
  border-radius: 16px;
  font-size: 13px;
  color: var(--color-button-font);
  background: var(--color-button-background);
  transition: opacity var(--anim-dur-fast) var(--anim-ease-smooth), transform var(--anim-dur-fast) var(--anim-ease-standard);
}
.more-btn:hover {
  background: var(--color-button-background-hover);
}

/* LX 空白页：标题 + chip 流式布局 */
.blank {
  padding: 5px;
}
.group + .group {
  margin-top: 14px;
}
.group-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 5px 5px 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--color-font);
}
.title {
  display: flex;
  align-items: center;
  gap: 6px;
}
.clear {
  display: flex;
  color: var(--color-font-label);
  opacity: 0.45;
  transition: opacity 0.2s ease;
}
.clear:hover {
  opacity: 1;
  color: var(--color-font);
}
.group-body {
  margin: 0;
}
.chip {
  display: inline-block;
  max-width: 150px;
  margin: 3px 5px;
  padding: 7px 10px;
  border-radius: 5px;
  font-size: 13px;
  color: var(--color-button-font);
  background-color: var(--color-button-background);
  transition:
    opacity var(--anim-dur-fast) var(--anim-ease-smooth),
    background-color var(--anim-dur-fast) var(--anim-ease-smooth);
  vertical-align: middle;
}
.chip:hover {
  background-color: var(--color-button-background-hover);
}
.chip:active {
  background-color: var(--color-button-background-active);
}
</style>
