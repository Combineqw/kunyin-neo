<script setup lang="ts">
import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useSettingsStore } from '../../stores/settings'
import { usePlayerStore } from '../../stores/player'
import {
  AI_QUALITY_CANDIDATES,
  QUALITY_IDS,
  QUALITY_NAMES,
  blockedQualityIds,
  qualityFallbackOrder,
  type EqualizerPresetId,
  type QualityId
} from '@common'
import {
  EQ_FREQUENCIES,
  EQ_GAIN_MAX,
  EQ_GAIN_MIN,
  EQUALIZER_PRESETS
} from '../../audio/audioGraph'
import {
  AUTOEQ_PROFILES,
  equalizerProfileFromGains,
  normalizeEqualizerProfile,
  type EqualizerFilter,
  type EqualizerProfile
} from '@common'
import BaseBtn from '../../components/BaseBtn.vue'
import BaseCheckbox from '../../components/BaseCheckbox.vue'
import BaseSelect from '../../components/BaseSelect.vue'

const store = useSettingsStore()
const player = usePlayerStore()
const { settings } = storeToRefs(store)
const previewing = ref(false)
const previewError = ref('')
const responseCurve = ref<number[]>([])

const blocked = computed(() => blockedQualityIds(settings.value))

// 被屏蔽的档位不作为首选（当前值若已被屏蔽仍保留在列表里，避免下拉显示空白）
const qualityList = computed(() =>
  QUALITY_IDS.filter(
    (q) => !blocked.value.includes(q) || q === settings.value.player.preferredQuality
  ).map((q) => ({ id: q as string, label: QUALITY_NAMES[q] }))
)

const aiList = AI_QUALITY_CANDIDATES.map((q) => ({ id: q, label: QUALITY_NAMES[q] }))

function setQuality(id: string): void {
  void store.update({ player: { preferredQuality: id as QualityId } })
}

/** 首选音质落在被屏蔽档时，降到最近的未屏蔽可用档，免得每次播放都白试一轮 */
function fixPreferred(blockedIds: readonly string[]): QualityId | null {
  const cur = settings.value.player.preferredQuality
  if (!blockedIds.includes(cur)) return null
  const next = qualityFallbackOrder(cur, undefined, blockedIds)[0]
  return (next as QualityId) ?? 'flac'
}

function setBlockAi(on: boolean): void {
  const next = fixPreferred(on ? settings.value.quality.aiQualities : [])
  void store.update({
    quality: { blockAi: on },
    ...(next ? { player: { preferredQuality: next } } : {})
  })
}

function toggleAiQuality(id: QualityId, on: boolean): void {
  const cur = settings.value.quality.aiQualities
  const list = on ? [...new Set([...cur, id])] : cur.filter((q) => q !== id)
  const next = settings.value.quality.blockAi ? fixPreferred(list) : null
  void store.update({
    quality: { aiQualities: list },
    ...(next ? { player: { preferredQuality: next } } : {})
  })
}

const presetList: Array<{ id: EqualizerPresetId; label: string }> = [
  { id: 'flat', label: '原声' },
  { id: 'bass', label: '低音增强' },
  { id: 'vocal', label: '人声突出' },
  { id: 'treble', label: '高音增强' },
  { id: 'custom', label: '自定义' }
]

function setEqualizerEnabled(enabled: boolean): void {
  void store.update({ player: { equalizerEnabled: enabled } })
}

function setBand(index: number, event: Event): void {
  const value = Number((event.target as HTMLInputElement).value)
  const gains = [...settings.value.player.equalizerGains]
  gains[index] = Math.max(EQ_GAIN_MIN, Math.min(EQ_GAIN_MAX, value))
  void store.update({
    player: {
      equalizerPreset: 'custom',
      equalizerGains: gains,
      equalizerPreampDb: 0,
      equalizerFilters: [],
      equalizerProfile: {
        format: 'kunyin-eq',
        version: 1,
        name: '自定义调音',
        author: '坤音neo',
        source: 'local'
      }
    }
  })
}

function setPreset(id: string): void {
  const preset = id as EqualizerPresetId
  if (preset === 'custom') return
  void store.update({
    player: {
      equalizerPreset: preset,
      equalizerGains: [...EQUALIZER_PRESETS[preset]],
      equalizerPreampDb: 0,
      equalizerFilters: [],
      equalizerProfile: {
        format: 'kunyin-eq',
        version: 1,
        name: presetList.find((item) => item.id === preset)?.label ?? '原声',
        author: '坤音neo',
        source: 'local'
      }
    }
  })
}

function selectAutoEqProfile(event: Event): void {
  const name = (event.target as HTMLSelectElement).value
  const profile = AUTOEQ_PROFILES.find((item) => item.name === name)
  if (!profile) return
  void store.update({
    player: {
      equalizerEnabled: true,
      ...profilePatch(profile)
    }
  })
}

function resetEqualizer(): void {
  void store.update({
    player: {
      equalizerEnabled: false,
      equalizerPreset: 'flat',
      equalizerGains: [...EQUALIZER_PRESETS.flat],
      equalizerPreampDb: 0,
      equalizerFilters: [],
      equalizerProfile: {
        format: 'kunyin-eq',
        version: 1,
        name: '原声',
        author: '坤音neo',
        source: 'local'
      }
    }
  })
}

function setSrsEnabled(enabled: boolean): void {
  void store.update({ player: { srsEnabled: enabled } })
}

function setSrsIntensity(event: Event): void {
  const value = Math.max(0, Math.min(100, Number((event.target as HTMLInputElement).value)))
  void store.update({ player: { srsIntensity: value } })
}

function setSrsValue(
  key: 'srsBass' | 'srsVoice' | 'srsTreble' | 'srsSpace',
  event: Event
): void {
  const value = Math.max(0, Math.min(100, Number((event.target as HTMLInputElement).value)))
  if (key === 'srsBass') void store.update({ player: { srsBass: value } })
  if (key === 'srsVoice') void store.update({ player: { srsVoice: value } })
  if (key === 'srsTreble') void store.update({ player: { srsTreble: value } })
  if (key === 'srsSpace') void store.update({ player: { srsSpace: value } })
}

function setSrsLimiter(enabled: boolean): void {
  void store.update({ player: { srsLimiter: enabled } })
}

async function importIrs(): Promise<void> {
  try {
    const result = await window.api.irs.importProfile()
    if (!result) return
    if (result.profile.durationMs > 3000) {
      const seconds = (result.profile.durationMs / 1000).toFixed(2)
      const confirmed = window.confirm(
        `该 IRS 脉冲响应长度为 ${seconds} 秒，可能消耗较高 CPU。仍要加载吗？`
      )
      if (!confirmed) return
    }
    const profiles = [
      ...settings.value.player.irsProfiles.filter((item) => item.id !== result.profile.id),
      result.profile
    ]
    await store.update({
      player: {
        irsProfiles: profiles,
        irsProfileId: result.profile.id,
        irsEnabled: true
      }
    })
  } catch (error) {
    window.alert(error instanceof Error ? error.message : '导入 IRS 失败')
  }
}

function selectIrs(event: Event): void {
  const id = (event.target as HTMLSelectElement).value
  void store.update({ player: { irsProfileId: id, irsEnabled: !!id } })
}

function renameIrs(): void {
  const id = settings.value.player.irsProfileId
  const current = settings.value.player.irsProfiles.find((item) => item.id === id)
  if (!current) return
  const name = window.prompt('请输入 IRS 预设名称', current.name)?.trim()
  if (!name || name === current.name) return
  void store.update({
    player: {
      irsProfiles: settings.value.player.irsProfiles.map((item) =>
        item.id === id ? { ...item, name: name.slice(0, 120) } : item
      )
    }
  })
}

function deleteIrs(): void {
  const id = settings.value.player.irsProfileId
  if (!id) return
  const current = settings.value.player.irsProfiles.find((item) => item.id === id)
  if (!current || !window.confirm(`确定删除 IRS 预设“${current.name}”吗？`)) return
  const profiles = settings.value.player.irsProfiles.filter((item) => item.id !== id)
  void store.update({ player: { irsProfiles: profiles, irsProfileId: profiles[0]?.id ?? '', irsEnabled: false } })
}

function setIrsEnabled(enabled: boolean): void {
  void store.update({ player: { irsEnabled: enabled } })
}

function setIrsWet(event: Event): void {
  const value = Math.max(0, Math.min(100, Number((event.target as HTMLInputElement).value)))
  void store.update({ player: { irsWetPercent: value, irsDryPercent: 100 - value } })
  responseCurve.value = player.getIrsResponseCurve()
}

async function previewEffect(): Promise<void> {
  if (previewing.value) return
  previewing.value = true
  previewError.value = ''
  try {
    responseCurve.value = await player.previewAudioEffect()
  } catch (error) {
    previewError.value = error instanceof Error ? error.message : '效果预览失败'
  } finally {
    previewing.value = false
  }
}

const curvePoints = computed(() => {
  const values = responseCurve.value
  if (!values.length) return ''
  return values.map((value, index) => `${(index / Math.max(1, values.length - 1)) * 100},${100 - value * 86 - 7}`).join(' ')
})

function setFadeEnabled(enabled: boolean): void {
  void store.update({ player: { fadeEnabled: enabled } })
}

const profileSourceLabel = computed(() => {
  const source = settings.value.player.equalizerProfile.source
  return source === 'official' ? '官方目录' : source === 'community' ? '社区' : '本地'
})

function currentEqualizerProfile(): EqualizerProfile {
  const meta = settings.value.player.equalizerProfile
  const base = equalizerProfileFromGains(settings.value.player.equalizerGains, {
    ...meta,
    name: meta.name || '自定义调音',
    author: meta.author || '坤音neo',
    source: 'local'
  })
  return normalizeEqualizerProfile({
    ...base,
    preampDb: settings.value.player.equalizerPreampDb,
    filters: settings.value.player.equalizerFilters
  })
}

function profilePatch(profile: EqualizerProfile): {
  equalizerPreset: 'custom'
  equalizerGains: number[]
  equalizerPreampDb: number
  equalizerFilters: EqualizerFilter[]
  equalizerProfile: typeof settings.value.player.equalizerProfile
} {
  return {
    equalizerPreset: 'custom',
    equalizerGains: profile.bands.map((band) => band.gainDb),
    equalizerPreampDb: profile.preampDb ?? 0,
    equalizerFilters: profile.filters ?? [],
    equalizerProfile: {
      format: profile.format,
      version: profile.version,
      name: profile.name,
      author: profile.author,
      description: profile.description,
      preampDb: profile.preampDb,
      source: profile.source,
      sourceUrl: profile.sourceUrl,
      checksum: profile.checksum
    }
  }
}

async function importEqualizer(): Promise<void> {
  try {
    const result = await window.api.equalizer.importProfile()
    if (!result) return
    const profile = result.profile
    await store.update({
      player: {
        equalizerEnabled: true,
        ...profilePatch(profile),
        equalizerCustomProfiles: [
          ...settings.value.player.equalizerCustomProfiles.filter((item) => item.name !== profile.name),
          profile
        ]
      }
    })
  } catch (error) {
    window.alert(error instanceof Error ? error.message : '导入调音文件失败')
  }
}

async function saveCustomProfile(): Promise<void> {
  const current = currentEqualizerProfile()
  const name = window.prompt('请输入自定义调音预设名称', current.name)
  if (!name?.trim()) return
  const profile = normalizeEqualizerProfile({ ...current, name: name.trim(), source: 'local' })
  await store.update({
    player: {
      ...profilePatch(profile),
      equalizerCustomProfiles: [
        ...settings.value.player.equalizerCustomProfiles.filter((item) => item.name !== profile.name),
        profile
      ]
    }
  })
}

function selectCustomProfile(event: Event): void {
  const name = (event.target as HTMLSelectElement).value
  const profile = settings.value.player.equalizerCustomProfiles.find((item) => item.name === name)
  if (!profile) return
  void store.update({ player: { equalizerEnabled: true, ...profilePatch(profile) } })
}

function deleteCustomProfile(): void {
  const name = settings.value.player.equalizerProfile.name
  const profile = settings.value.player.equalizerCustomProfiles.find((item) => item.name === name)
  if (!profile || !window.confirm(`确定删除导入或保存的调音预设“${profile.name}”吗？`)) return
  void store.update({
    player: {
      equalizerEnabled: false,
      equalizerPreset: 'flat',
      equalizerGains: [...EQUALIZER_PRESETS.flat],
      equalizerPreampDb: 0,
      equalizerFilters: [],
      equalizerProfile: {
        format: 'kunyin-eq',
        version: 1,
        name: '原声',
        author: '坤音neo',
        source: 'local'
      },
      equalizerCustomProfiles: settings.value.player.equalizerCustomProfiles.filter(
        (item) => item.name !== name
      )
    }
  })
}

async function exportEqualizer(): Promise<void> {
  try {
    const fileName = await window.api.equalizer.exportProfile(currentEqualizerProfile())
    if (fileName) window.alert(`已导出调音文件：${fileName}`)
  } catch (error) {
    window.alert(error instanceof Error ? error.message : '导出调音文件失败')
  }
}

function setFadeDuration(event: Event): void {
  const value = Number((event.target as HTMLInputElement).value)
  void store.update({ player: { fadeDurationMs: Math.max(0, Math.min(1000, value)) } })
}
</script>

<template>
  <dt id="play">播放设置</dt>
  <dd>
    <h3 id="play_quality">首选音质</h3>
    <div>
      <BaseSelect
        :model-value="settings.player.preferredQuality"
        :list="qualityList"
        @update:model-value="setQuality"
      />
    </div>
  </dd>

  <dd>
    <h3 id="play_audio">音频处理</h3>
    <div>
      <BaseCheckbox
        id="setting_equalizer_enabled"
        :model-value="settings.player.equalizerEnabled"
        label="启用十段均衡器"
        @update:model-value="setEqualizerEnabled($event as boolean)"
      />
      <div class="eq-toolbar gap-top">
        <label for="setting_equalizer_preset">均衡器预设</label>
        <select
          id="setting_equalizer_preset"
          :value="settings.player.equalizerPreset"
          :disabled="!settings.player.equalizerEnabled"
          @change="setPreset(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="preset in presetList" :key="preset.id" :value="preset.id">
            {{ preset.label }}
          </option>
        </select>
        <select
          class="autoeq-select"
          :value="settings.player.equalizerProfile.name"
          :disabled="!settings.player.equalizerEnabled"
          aria-label="选择内置耳机校准"
          @change="selectAutoEqProfile"
        >
          <option value="">内置耳机校准</option>
          <option v-for="profile in AUTOEQ_PROFILES" :key="profile.name" :value="profile.name">
            {{ profile.name }}
          </option>
        </select>
        <select
          v-if="settings.player.equalizerCustomProfiles.length"
          :value="settings.player.equalizerProfile.name"
          :disabled="!settings.player.equalizerEnabled"
          aria-label="选择自定义调音预设"
          @change="selectCustomProfile"
        >
          <option value="">自定义预设</option>
          <option
            v-for="profile in settings.player.equalizerCustomProfiles"
            :key="profile.name"
            :value="profile.name"
          >
            {{ profile.name }}
          </option>
        </select>
        <BaseBtn min outline @click="resetEqualizer">重置</BaseBtn>
        <BaseBtn min outline @click="importEqualizer">导入调音</BaseBtn>
        <BaseBtn min outline @click="saveCustomProfile">保存预设</BaseBtn>
        <BaseBtn
          v-if="settings.player.equalizerCustomProfiles.some((item) => item.name === settings.player.equalizerProfile.name)"
          min
          outline
          @click="deleteCustomProfile"
        >删除预设</BaseBtn>
        <BaseBtn min outline @click="exportEqualizer">导出 APO</BaseBtn>
      </div>
      <p class="eq-profile-meta">
        当前调音：{{ settings.player.equalizerProfile.name }} ·
        {{ settings.player.equalizerProfile.author }} · {{ profileSourceLabel }}
        <template v-if="settings.player.equalizerFilters.length">
          · 前置 {{ settings.player.equalizerPreampDb }} dB · {{ settings.player.equalizerFilters.length }} 个动态滤镜
        </template>
      </p>
      <div class="eq-grid" :class="{ disabled: !settings.player.equalizerEnabled }">
        <label v-for="(frequency, index) in EQ_FREQUENCIES" :key="frequency" class="eq-band">
          <span>{{ frequency >= 1000 ? `${frequency / 1000}k` : frequency }} Hz</span>
          <input
            type="range"
            :min="EQ_GAIN_MIN"
            :max="EQ_GAIN_MAX"
            step="1"
            :value="settings.player.equalizerGains[index] ?? 0"
            :disabled="!settings.player.equalizerEnabled"
            :aria-label="`${frequency} Hz 增益`"
            @input="setBand(index, $event)"
          />
          <b>{{ settings.player.equalizerGains[index] ?? 0 }} dB</b>
        </label>
      </div>
      <div class="srs-panel gap-top aurora-divider">
        <BaseCheckbox
          id="setting_srs_enabled"
          :model-value="settings.player.srsEnabled"
          label="启用 SRS 风格空间音效"
          @update:model-value="setSrsEnabled($event as boolean)"
        />
        <p class="srs-note">空间扩展、低音、人声和高频增强的轻量本地处理，不依赖网络。</p>
        <div class="srs-grid" :class="{ disabled: !settings.player.srsEnabled }">
          <label>
            <span>SRS 总强度 <b>{{ settings.player.srsIntensity }}%</b></span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              :value="settings.player.srsIntensity"
              :disabled="!settings.player.srsEnabled"
              @input="setSrsIntensity"
            />
          </label>
          <label>
            <span>低音增强 <b>{{ settings.player.srsBass }}</b></span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              :value="settings.player.srsBass"
              :disabled="!settings.player.srsEnabled"
              @input="setSrsValue('srsBass', $event)"
            />
          </label>
          <label>
            <span>人声清晰度 <b>{{ settings.player.srsVoice }}</b></span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              :value="settings.player.srsVoice"
              :disabled="!settings.player.srsEnabled"
              @input="setSrsValue('srsVoice', $event)"
            />
          </label>
          <label>
            <span>高频空气感 <b>{{ settings.player.srsTreble }}</b></span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              :value="settings.player.srsTreble"
              :disabled="!settings.player.srsEnabled"
              @input="setSrsValue('srsTreble', $event)"
            />
          </label>
          <label>
            <span>空间宽度 <b>{{ settings.player.srsSpace }}</b></span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              :value="settings.player.srsSpace"
              :disabled="!settings.player.srsEnabled"
              @input="setSrsValue('srsSpace', $event)"
            />
          </label>
        </div>
        <BaseCheckbox
          id="setting_srs_limiter"
          :model-value="settings.player.srsLimiter"
          :disabled="!settings.player.srsEnabled"
          label="启用输出保护，降低增强后的削波风险"
          @update:model-value="setSrsLimiter($event as boolean)"
        />
      </div>
      <div class="irs-panel gap-top aurora-divider">
        <div class="irs-heading">
          <div>
            <strong>脉冲响应卷积</strong>
            <p>导入个人 IRS 文件后，以卷积方式叠加耳机、房间或设备的频响特征。</p>
          </div>
          <BaseCheckbox
            id="setting_irs_enabled"
            :model-value="settings.player.irsEnabled"
            :disabled="!settings.player.irsProfileId"
            label="启用"
            @update:model-value="setIrsEnabled($event as boolean)"
          />
        </div>
        <div class="irs-toolbar gap-top">
          <select
            :value="settings.player.irsProfileId"
            aria-label="选择脉冲响应文件"
            @change="selectIrs"
          >
            <option value="">选择已导入的脉冲响应</option>
            <option v-for="profile in settings.player.irsProfiles" :key="profile.id" :value="profile.id">
              {{ profile.name }} · {{ profile.sampleRate }} Hz · {{ profile.durationMs }} ms
            </option>
          </select>
          <BaseBtn min outline @click="importIrs">导入 IRS</BaseBtn>
          <BaseBtn min outline :disabled="previewing || !settings.player.irsProfileId" @click="previewEffect">
            {{ previewing ? '试听中…' : '试听响应' }}
          </BaseBtn>
          <BaseBtn min outline :disabled="!settings.player.irsProfileId" @click="renameIrs">重命名</BaseBtn>
          <BaseBtn min outline :disabled="!settings.player.irsProfileId" @click="deleteIrs">删除</BaseBtn>
        </div>
        <label class="irs-range gap-top" :class="{ disabled: !settings.player.irsEnabled }">
          <span>混合比例 <b>原声 {{ 100 - settings.player.irsWetPercent }}% / 卷积 {{ settings.player.irsWetPercent }}%</b></span>
          <input
            type="range"
            min="0"
            max="100"
            step="1"
            :value="settings.player.irsWetPercent"
            :disabled="!settings.player.irsEnabled"
            @input="setIrsWet"
          />
        </label>
        <div v-if="responseCurve.length" class="irs-curve" aria-label="脉冲响应频响预览">
          <svg viewBox="0 0 100 100" preserveAspectRatio="none" role="img">
            <polyline :points="curvePoints" fill="none" stroke="currentColor" stroke-width="2" vector-effect="non-scaling-stroke" />
          </svg>
          <span>20 Hz</span><span>20 kHz</span>
        </div>
        <p v-else class="irs-empty">导入并选择一个 IRS 文件后，可在这里试听并查看频响预览。</p>
        <p v-if="previewError" class="irs-error">{{ previewError }}</p>
      </div>
      <BaseCheckbox
        id="setting_fade_enabled"
        :model-value="settings.player.fadeEnabled"
        label="切歌、暂停和恢复播放时使用淡入淡出"
        @update:model-value="setFadeEnabled($event as boolean)"
      />
      <label class="fade-range gap-top">
        <span>淡入淡出时长 <b>{{ settings.player.fadeDurationMs }} ms</b></span>
        <input
          type="range"
          min="0"
          max="1000"
          step="50"
          :value="settings.player.fadeDurationMs"
          :disabled="!settings.player.fadeEnabled"
          @input="setFadeDuration"
        />
      </label>
    </div>
  </dd>

  <dd>
    <h3 id="play_block_ai">AI 音质</h3>
    <div>
      <BaseCheckbox
        id="setting_quality_block_ai"
        :model-value="settings.quality.blockAi"
        label="屏蔽 AI 音质（播放与下载均跳过这些版本）"
        @update:model-value="setBlockAi($event as boolean)"
      />
      <div v-for="q in aiList" :key="q.id" class="gap-top gap-left">
        <BaseCheckbox
          :id="`setting_quality_ai_${q.id}`"
          :model-value="settings.quality.aiQualities.includes(q.id)"
          :disabled="!settings.quality.blockAi"
          :label="q.label"
          @update:model-value="toggleAiQuality(q.id, $event as boolean)"
        />
      </div>
    </div>
  </dd>
</template>

<style scoped>
.eq-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.eq-toolbar label,
.fade-range > span {
  color: var(--color-font-label);
  font-size: 12px;
}
.eq-profile-meta {
  margin: -4px 0 14px;
  color: var(--color-font-label);
  font-size: 11px;
}
.eq-toolbar select {
  min-width: 116px;
  padding: 5px 8px;
  border: 1px solid var(--color-primary-alpha-700);
  border-radius: 6px;
  color: var(--color-font);
  background: var(--color-main-background);
}
.eq-grid {
  display: grid;
  grid-template-columns: repeat(10, minmax(38px, 1fr));
  gap: 10px;
  margin: 16px 0 18px;
}
.eq-grid.disabled {
  opacity: 0.45;
}
.eq-band {
  display: flex;
  min-width: 0;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--color-font-label);
  font-size: 10px;
}
.eq-band input {
  width: 86px;
  transform: rotate(-90deg);
  margin: 30px 0;
}
.eq-band b {
  min-height: 14px;
  color: var(--color-font);
  font-size: 10px;
  font-weight: 600;
  white-space: nowrap;
}
.srs-panel {
  max-width: 640px;
  padding: 12px 0 4px;
  border-top: 1px solid var(--color-primary-alpha-200);
}
.srs-note {
  margin: 7px 0 12px;
  color: var(--color-font-label);
  font-size: 11px;
}
.srs-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(180px, 1fr));
  gap: 12px 18px;
  margin: 12px 0 14px;
}
.srs-grid.disabled {
  opacity: 0.45;
}
.srs-grid label {
  display: grid;
  gap: 6px;
  color: var(--color-font-label);
  font-size: 11px;
}
.srs-grid label span {
  display: flex;
  justify-content: space-between;
}
.srs-grid b {
  color: var(--color-font);
}
.irs-panel {
  max-width: 640px;
  padding: 12px 0 4px;
  border-top: 1px solid var(--color-primary-alpha-200);
}
.irs-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.irs-heading strong {
  color: var(--color-font);
  font-size: 13px;
}
.irs-heading p,
.irs-empty {
  margin: 5px 0 0;
  color: var(--color-font-label);
  font-size: 11px;
  line-height: 1.6;
}
.irs-toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.irs-toolbar select {
  min-width: 220px;
  padding: 5px 8px;
  border: 1px solid var(--color-primary-alpha-700);
  border-radius: 6px;
  color: var(--color-font);
  background: var(--color-main-background);
}
.irs-range {
  display: grid;
  gap: 8px;
  max-width: 420px;
  color: var(--color-font-label);
  font-size: 11px;
}
.irs-range.disabled {
  opacity: 0.45;
}
.irs-curve {
  position: relative;
  width: min(100%, 520px);
  height: 120px;
  margin-top: 14px;
  border: 1px solid var(--color-border);
  background: repeating-linear-gradient(
    to bottom,
    transparent 0,
    transparent 29px,
    var(--color-primary-alpha-900) 30px
  );
  color: var(--color-primary-font);
}
.irs-curve svg {
  width: 100%;
  height: 100%;
  display: block;
}
.irs-curve span {
  position: absolute;
  bottom: 4px;
  color: var(--color-font-label);
  font-size: 9px;
}
.irs-curve span:first-of-type { left: 5px; }
.irs-curve span:last-of-type { right: 5px; }
.irs-error {
  margin: 8px 0 0;
  color: var(--color-danger, #b3261e);
  font-size: 11px;
}
.fade-range {
  display: grid;
  gap: 8px;
  max-width: 420px;
}
.fade-range > span {
  display: flex;
  justify-content: space-between;
}
.fade-range b {
  color: var(--color-font);
}
@media (max-width: 760px) {
  .srs-grid {
    grid-template-columns: 1fr;
  }
  .eq-grid {
    grid-template-columns: repeat(5, minmax(48px, 1fr));
    row-gap: 18px;
  }
}
</style>
