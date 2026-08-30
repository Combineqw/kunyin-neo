<script setup lang="ts">
import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import {
  animationPackFromJson,
  animationPackToJson,
  DEFAULT_ANIMATION_PACKS,
  type AnimationPack,
  type AnimationPackId
} from '@common'
import AppIcon from '../../components/AppIcon.vue'
import BaseBtn from '../../components/BaseBtn.vue'
import BaseSelect from '../../components/BaseSelect.vue'
import { useSettingsStore } from '../../stores/settings'

const store = useSettingsStore()
const { settings } = storeToRefs(store)
const dragging = ref<{ curve: 'standard' | 'spring'; point: 0 | 1 } | null>(null)

const packOptions = [
  ...DEFAULT_ANIMATION_PACKS.map((pack) => ({ id: pack.id, label: pack.name })),
  { id: 'custom' as const, label: '自定义动效' }
]

const activePack = computed(() => settings.value.behavior.animationPackId)
const customPack = computed(() => settings.value.behavior.customAnimationPack)
const standardPoints = computed(() => readCurve(customPack.value.easeStandard))
const springPoints = computed(() => readCurve(customPack.value.easeSpring))

function readCurve(value: string): [[number, number], [number, number]] {
  const match = value.match(/cubic-bezier\(([-\d.]+),\s*([-\d.]+),\s*([-\d.]+),\s*([-\d.]+)\)/)
  if (!match) return [[0.32, 0.72], [0.64, 1]]
  return [
    [Number(match[1]), Number(match[2])],
    [Number(match[3]), Number(match[4])]
  ]
}

function curveValue(points: [[number, number], [number, number]]): string {
  return `cubic-bezier(${points[0][0].toFixed(2)}, ${points[0][1].toFixed(2)}, ${points[1][0].toFixed(2)}, ${points[1][1].toFixed(2)})`
}

function updateCustom(patch: Partial<AnimationPack>): void {
  void store.update({ behavior: { animationPackId: 'custom', customAnimationPack: patch } })
}

function choosePack(id: AnimationPackId): void {
  void store.update({ behavior: { animationPackId: id } })
}

function updatePoint(curve: 'standard' | 'spring', point: 0 | 1, x: number, y: number): void {
  const next = curve === 'standard' ? standardPoints.value.map((v) => [...v]) : springPoints.value.map((v) => [...v])
  next[point] = [Math.max(0, Math.min(1, x)), Math.max(-0.5, Math.min(1.5, y))]
  updateCustom({ [curve === 'standard' ? 'easeStandard' : 'easeSpring']: curveValue(next as [[number, number], [number, number]]) })
}

function onPointerMove(event: PointerEvent, element: SVGSVGElement): void {
  if (!dragging.value) return
  const rect = element.getBoundingClientRect()
  const x = (event.clientX - rect.left) / rect.width
  const y = 1 - (event.clientY - rect.top) / rect.height
  updatePoint(dragging.value.curve, dragging.value.point, x, y)
}

function startDrag(curve: 'standard' | 'spring', point: 0 | 1, event: PointerEvent): void {
  const element = event.currentTarget as SVGCircleElement
  element.setPointerCapture(event.pointerId)
  dragging.value = { curve, point }
}

function stopDrag(): void {
  dragging.value = null
}

function pointToSvg(point: [number, number]): string {
  return `${point[0] * 240},${(1 - point[1]) * 120}`
}

function curvePath(points: [[number, number], [number, number]]): string {
  return `M 0 120 C ${pointToSvg(points[0])} ${pointToSvg(points[1])} 240 0`
}

function ms(value: string): number {
  const parsed = Number.parseFloat(value)
  return Number.isFinite(parsed) ? Math.round(parsed * 1000) : 350
}

function updateDuration(key: 'durationFast' | 'durationBase' | 'durationSlow', event: Event): void {
  const seconds = Number((event.target as HTMLInputElement).value) / 1000
  updateCustom({ [key]: `${seconds.toFixed(2)}s` })
}

function setDefaultCurve(curve: 'standard' | 'spring'): void {
  const pack = DEFAULT_ANIMATION_PACKS[0]
  updateCustom({ [curve === 'standard' ? 'easeStandard' : 'easeSpring']: curve === 'standard' ? pack.easeStandard : pack.easeSpring })
}

async function importPack(): Promise<void> {
  try {
    const value = await window.api.animation.importPack()
    if (!value) return
    const pack = animationPackFromJson(value)
    void store.update({ behavior: { animationPackId: 'custom', customAnimationPack: pack } })
  } catch (error) {
    window.alert(error instanceof Error ? error.message : '导入动效包失败')
  }
}

async function exportPack(): Promise<void> {
  try {
    const fileName = await window.api.animation.exportPack(animationPackToJson(customPack.value))
    if (fileName) window.alert(`已导出动效包：${fileName}`)
  } catch (error) {
    window.alert(error instanceof Error ? error.message : '导出动效包失败')
  }
}
</script>

<template>
  <dt id="animation">动效实验室</dt>
  <dd>
    <h3>动效包</h3>
    <div class="pack-row">
      <BaseSelect :model-value="activePack" :list="packOptions" @update:model-value="choosePack($event as AnimationPackId)" />
      <BaseBtn min outline @click="importPack"><AppIcon name="upload" :size="14" />导入</BaseBtn>
      <BaseBtn min outline @click="exportPack"><AppIcon name="download" :size="14" />导出</BaseBtn>
    </div>
    <p class="hint">切换预设只改变界面动效参数，不会重建音频图或中断当前播放。</p>
  </dd>
  <dd>
    <h3>缓动曲线 <span class="hint">拖动控制点实时调整自定义动效</span></h3>
    <div class="curves">
      <div class="curve-editor">
        <div class="curve-label"><span>标准缓动</span><button type="button" class="pressable" @click="setDefaultCurve('standard')">恢复默认</button></div>
        <svg class="curve" viewBox="0 0 240 120" @pointermove="onPointerMove($event, ($event.currentTarget as SVGSVGElement))" @pointerup="stopDrag" @pointerleave="stopDrag">
          <path class="grid" d="M0 0H240M0 60H240M0 120H240M60 0V120M120 0V120M180 0V120" />
          <path class="curve-line" :d="curvePath(standardPoints)" />
          <line class="handle" x1="0" y1="120" :x2="pointToSvg(standardPoints[0]).split(',')[0]" :y2="pointToSvg(standardPoints[0]).split(',')[1]" />
          <line class="handle" x1="240" y1="0" :x2="pointToSvg(standardPoints[1]).split(',')[0]" :y2="pointToSvg(standardPoints[1]).split(',')[1]" />
          <circle v-for="(point, index) in standardPoints" :key="index" class="control" :cx="pointToSvg(point).split(',')[0]" :cy="pointToSvg(point).split(',')[1]" r="7" @pointerdown="startDrag('standard', index as 0 | 1, $event)" />
        </svg>
        <code>{{ customPack.easeStandard }}</code>
      </div>
      <div class="curve-editor">
        <div class="curve-label"><span>弹性缓动</span><button type="button" class="pressable" @click="setDefaultCurve('spring')">恢复默认</button></div>
        <svg class="curve" viewBox="0 0 240 120" @pointermove="onPointerMove($event, ($event.currentTarget as SVGSVGElement))" @pointerup="stopDrag" @pointerleave="stopDrag">
          <path class="grid" d="M0 0H240M0 60H240M0 120H240M60 0V120M120 0V120M180 0V120" />
          <path class="curve-line" :d="curvePath(springPoints)" />
          <line class="handle" x1="0" y1="120" :x2="pointToSvg(springPoints[0]).split(',')[0]" :y2="pointToSvg(springPoints[0]).split(',')[1]" />
          <line class="handle" x1="240" y1="0" :x2="pointToSvg(springPoints[1]).split(',')[0]" :y2="pointToSvg(springPoints[1]).split(',')[1]" />
          <circle v-for="(point, index) in springPoints" :key="index" class="control" :cx="pointToSvg(point).split(',')[0]" :cy="pointToSvg(point).split(',')[1]" r="7" @pointerdown="startDrag('spring', index as 0 | 1, $event)" />
        </svg>
        <code>{{ customPack.easeSpring }}</code>
      </div>
    </div>
  </dd>
  <dd>
    <h3>动画时长</h3>
    <div class="duration-grid">
      <label v-for="item in [{ key: 'durationFast', name: '快' }, { key: 'durationBase', name: '中' }, { key: 'durationSlow', name: '慢' }]" :key="item.key">
        <span>{{ item.name }} <b>{{ ms(customPack[item.key as keyof AnimationPack]) }} ms</b></span>
        <input type="range" min="80" max="900" step="10" :value="ms(customPack[item.key as keyof AnimationPack])" @input="updateDuration(item.key as 'durationFast' | 'durationBase' | 'durationSlow', $event)" />
      </label>
    </div>
  </dd>
  <dd>
    <h3>实时预览</h3>
    <div class="preview" :style="{ '--preview-duration': customPack.durationBase, '--preview-ease': customPack.easeSpring }">
      <div class="preview-bar"><span class="preview-cover"></span><span class="preview-title">正在播放 · 动效试听</span><AppIcon name="play" :size="16" /></div>
      <div class="preview-list">
        <div v-for="index in 3" :key="index" class="preview-item" :style="{ '--preview-delay': `${index * 50}ms` }"><span class="dot"></span><span>示例列表项 {{ index }}</span><span class="line"></span></div>
      </div>
    </div>
  </dd>
</template>

<style scoped>
.pack-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.pack-row :deep(.select) { min-width: 190px; }
.pack-row .btn { display: inline-flex; align-items: center; gap: 6px; }
.curves { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px; }
.curve-label { display: flex; justify-content: space-between; align-items: center; margin-bottom: 7px; font-size: 12px; }
.curve-label button { color: var(--color-primary); cursor: pointer; font-size: 11px; }
.curve { width: 100%; min-height: 130px; overflow: visible; border: 1px solid var(--color-border); border-radius: 8px; background: color-mix(in srgb, var(--color-main-background) 86%, var(--color-primary) 14%); touch-action: none; }
.grid { fill: none; stroke: var(--color-primary-alpha-900); stroke-width: 1; }
.handle { stroke: var(--color-primary-alpha-500); stroke-width: 1.5; stroke-dasharray: 4 3; }
.curve-line { fill: none; stroke: var(--color-primary); stroke-width: 3; }
.control { fill: var(--color-primary); stroke: var(--color-main-background); stroke-width: 3; cursor: grab; }
.control:active { cursor: grabbing; }
code { display: block; margin-top: 7px; color: var(--color-font-label); font-size: 10px; white-space: nowrap; }
.duration-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 18px; }
.duration-grid label { display: grid; gap: 9px; color: var(--color-font-label); font-size: 11px; }
.duration-grid b { color: var(--color-font); font-weight: 600; }
.preview { display: grid; gap: 9px; padding: 14px; border-radius: 9px; overflow: hidden; background: color-mix(in srgb, var(--color-main-background) 82%, var(--color-primary) 18%); }
.preview-bar, .preview-item { display: flex; align-items: center; gap: 10px; min-height: 38px; padding: 0 11px; border-radius: 7px; background: var(--color-main-background); }
.preview-bar { animation: preview-in var(--preview-duration) var(--preview-ease) both; }
.preview-cover, .dot { flex: none; width: 20px; height: 20px; border-radius: 5px; background: var(--color-primary); }
.preview-title { flex: 1; color: var(--color-font); font-size: 12px; }
.preview-list { display: grid; gap: 5px; }
.preview-item { animation: preview-in var(--preview-duration) var(--preview-ease) var(--preview-delay) both; color: var(--color-font-label); font-size: 11px; }
.preview-item .line { width: 34%; height: 5px; margin-left: auto; border-radius: 99px; background: var(--color-primary-background); }
@keyframes preview-in { from { opacity: 0; transform: translate3d(0, 12px, 0) scale(0.98); } to { opacity: 1; transform: translate3d(0, 0, 0) scale(1); } }
@media (max-width: 700px) { .curves, .duration-grid { grid-template-columns: 1fr; } }
</style>
