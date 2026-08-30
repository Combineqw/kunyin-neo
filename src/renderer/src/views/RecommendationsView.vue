<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import type { DailyRecommendationEntry, MusicItem } from '@common'
import AppIcon from '../components/AppIcon.vue'
import SongRow from '../components/SongRow.vue'
import { useLibraryStore } from '../stores/library'
import { usePlayerStore } from '../stores/player'
// 心动模式静态图标（首页按钮使用；侧栏与设置目录使用 AppIcon 内联同名图标，视觉一致）
import heartModeIcon from '../assets/icons/heart-mode.svg'

const player = usePlayerStore()
const library = useLibraryStore()
const { current } = storeToRefs(player)
const entries = ref<DailyRecommendationEntry[]>([])
const generatedAt = ref(0)
const loading = ref(false)
const message = ref('')

const favoriteCount = computed(() => library.favoriteKeys.size)
const playCount = computed(() => entries.value.reduce((sum, entry) => sum + entry.song.playCount, 0))
const isSparse = computed(() => favoriteCount.value < 20 || playCount.value < 50)

function formatTime(value: number): string {
  if (!value) return '--'
  return new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit' }).format(value)
}

function reason(entry: DailyRecommendationEntry): string {
  if (entry.reason === 'favorite') return '因为你喜欢过这首歌或相近的音乐'
  if (entry.reason === 'completion') return `因为你常常完整听完 ${entry.song.item.artist} 的歌曲`
  return '为你随机发现的本地曲库歌曲'
}

async function load(force = false): Promise<void> {
  loading.value = true
  message.value = ''
  try {
    const result = await window.api.recommendation.daily(force)
    entries.value = result.entries
    generatedAt.value = result.generatedAt
    if (!entries.value.length) message.value = '先播放或收藏几首歌曲，坤音neo就会开始为你生成推荐。'
  } catch {
    message.value = '本地推荐暂时无法生成，请稍后重试。'
  } finally {
    loading.value = false
  }
}

function isActive(item: MusicItem): boolean {
  return !!current.value && current.value.id === item.id && current.value.type === item.type
}

function play(entry: DailyRecommendationEntry): void {
  const list = entries.value.map((item) => item.song.item)
  player.playItem(entry.song.item, list, { source: { kind: 'single', name: '每日推荐' } })
}

async function saveAsPlaylist(): Promise<void> {
  if (!entries.value.length) return
  const name = `每日推荐 ${new Intl.DateTimeFormat('zh-CN', { month: '2-digit', day: '2-digit' }).format(generatedAt.value || Date.now())}`
  const playlistId = await library.createPlaylist(name)
  for (const entry of entries.value) await library.addToPlaylist(playlistId, entry.song.item)
  window.alert(`已固化为普通歌单「${name}」`)
}

async function startHeartbeat(): Promise<void> {
  const result = await player.startHeartbeatMode()
  if (result === 'insufficient-data') {
    if (window.confirm('喜欢的歌曲少于 20 首，推荐会偏随机。仍要开启心动模式吗？')) {
      const retry = await player.startHeartbeatMode(true)
      if (retry === 'no-match') window.alert('当前歌曲缺少可用于心动模式的本地推荐数据。')
    }
    return
  }
  if (result === 'disabled') window.alert('心动模式已在设置中关闭。')
  else if (result === 'no-match') window.alert('请先播放一首已写入本地推荐数据的歌曲。')
}

onMounted(() => void load())
</script>

<template>
  <section class="recommendations scroll">
    <header class="hero">
      <div>
        <span class="eyebrow">FOR YOU · LOCAL ONLY</span>
        <h1>每日推荐与心动模式</h1>
        <p>完全基于本地播放、完整收听和喜欢标记生成，不上传你的收听数据。</p>
      </div>
      <div class="hero-actions">
        <button class="primary" :disabled="!current" @click="startHeartbeat">
          <img class="hero-icon" :src="heartModeIcon" alt="" />
          从当前歌曲开启心动模式
        </button>
        <button class="secondary" :disabled="!entries.length" @click="void saveAsPlaylist">
          <AppIcon name="library" :size="16" />
          固化为歌单
        </button>
        <button class="secondary" :disabled="loading" @click="void load(true)">
          <AppIcon name="refresh" :size="16" />
          {{ loading ? '生成中…' : '刷新推荐' }}
        </button>
      </div>
    </header>

    <div v-if="isSparse" class="guide">
      <AppIcon name="info" :size="18" />
      <span>数据积累期：多播放、完整听完和收藏歌曲后，推荐会越来越贴近你的口味。</span>
    </div>

    <section class="daily">
      <div class="section-head">
        <div>
          <h2>今日 30 首</h2>
          <p>生成于 {{ formatTime(generatedAt) }} · 24 小时内保持稳定</p>
        </div>
        <span class="count">{{ entries.length }} 首</span>
      </div>
      <div v-if="loading" class="empty">正在整理你的本地收听信号…</div>
      <div v-else-if="message" class="empty">{{ message }}</div>
      <div v-else class="list">
        <article v-for="(entry, index) in entries" :key="`${entry.song.source}_${entry.song.songId}`" class="recommend-row">
          <SongRow
            :item="entry.song.item"
            :index="index"
            :active="isActive(entry.song.item)"
            @play="play(entry)"
          />
          <span class="reason">{{ reason(entry) }}</span>
        </article>
      </div>
    </section>
  </section>
</template>

<style scoped>
.recommendations { height: 100%; padding: 28px clamp(22px, 5vw, 64px) 42px; overflow-y: auto; }
.hero { display:flex; justify-content:space-between; gap:28px; padding:26px 30px; border:1px solid var(--color-border); border-radius:18px; background:linear-gradient(128deg, var(--color-primary-background), color-mix(in srgb, var(--color-main-background) 78%, var(--color-primary) 22%)); }
.eyebrow { display:block; color:var(--color-primary); font-size:10px; font-weight:700; letter-spacing:.14em; }
h1 { margin:6px 0; font-size:28px; letter-spacing:-.04em; } .hero p,.section-head p { margin:0; color:var(--color-font-label); font-size:12px; }
.hero-actions { display:flex; flex:none; align-items:center; gap:9px; }
button { display:flex; align-items:center; justify-content:center; gap:7px; min-height:34px; padding:0 13px; border-radius:9px; font-size:12px; cursor:pointer; }
.hero-icon { width:17px; height:17px; flex:none; color:inherit; }
.primary { color:var(--color-primary-font); background:var(--color-primary); } .secondary { color:var(--color-font); border:1px solid var(--color-border); background:var(--color-main-background); } button:disabled { opacity:.48; cursor:default; }
.guide { display:flex; gap:8px; align-items:center; margin:16px 0; padding:11px 13px; border-radius:10px; color:var(--color-font-label); background:var(--color-primary-background); font-size:12px; }
.daily { margin-top:25px; } .section-head { display:flex; align-items:end; justify-content:space-between; margin-bottom:9px; } h2 { margin:0 0 4px; font-size:18px; } .count { color:var(--color-font-label); font-size:12px; }
.list { border-top:1px solid var(--color-border); } .recommend-row { position:relative; border-bottom:1px solid var(--color-border); } .recommend-row :deep(.song-row) { padding-right:230px; } .reason { position:absolute; right:16px; top:50%; width:210px; transform:translateY(-50%); color:var(--color-font-label); font-size:11px; line-height:1.35; text-align:right; }
.empty { padding:28px 0; color:var(--color-font-label); font-size:13px; }
@media (max-width:900px) { .hero { flex-direction:column; } .hero-actions { flex-wrap:wrap; } .recommend-row :deep(.song-row) { padding-right:8px; } .reason { position:static; display:block; width:auto; margin:-7px 16px 10px 76px; transform:none; text-align:left; } }
</style>
