<script setup lang="ts">
import { computed } from 'vue'
import { useSettingsStore } from '../../stores/settings'

const settings = useSettingsStore()
const diversityText = computed(() => `${settings.settings.player.recommendationMaxPerArtist} 首`)

function update(patch: Partial<typeof settings.settings.player>): void {
  void settings.update({ player: patch })
}
</script>

<template>
  <dt>个性化推荐</dt>
  <dd>
    <h3>每日推荐 <span class="hint">仅使用本机数据</span></h3>
    <label class="p switch-row">
      <span><b>启用每日推荐</b><small>按播放完成度、跳过记录和喜欢标记生成 30 首歌曲</small></span>
      <input
        type="checkbox"
        :checked="settings.settings.player.dailyRecommendationEnabled"
        @change="update({ dailyRecommendationEnabled: ($event.target as HTMLInputElement).checked })"
      />
    </label>
  </dd>
  <dd>
    <h3>心动模式</h3>
    <label class="p switch-row">
      <span><b>启用心动模式</b><small>以当前歌曲为种子，在熟悉歌曲与新发现之间混合续播</small></span>
      <input
        type="checkbox"
        :checked="settings.settings.player.heartbeatEnabled"
        @change="update({ heartbeatEnabled: ($event.target as HTMLInputElement).checked })"
      />
    </label>
  </dd>
  <dd>
    <h3>推荐多样性</h3>
    <div class="p range-row">
      <span>同一歌手最多 {{ diversityText }}</span>
      <input
        type="range"
        min="1"
        max="3"
        step="1"
        :value="settings.settings.player.recommendationMaxPerArtist"
        @input="update({ recommendationMaxPerArtist: Number(($event.target as HTMLInputElement).value) })"
      />
    </div>
  </dd>
</template>

<style scoped>
.switch-row { display:flex; align-items:center; justify-content:space-between; gap:18px; cursor:pointer; }
.switch-row span { display:grid; gap:5px; } b { font-size:13px; font-weight:600; } small { color:var(--color-font-label); font-size:11px; line-height:1.4; } input[type='checkbox'] { width:18px; height:18px; accent-color:var(--color-primary); }
.range-row { display:grid; gap:12px; } input[type='range'] { width:min(100%, 350px); }
</style>
