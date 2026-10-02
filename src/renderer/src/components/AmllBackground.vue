<script setup lang="ts">
/**
 * AMLL MeshGradient 流体渐变背景（bg-render 内嵌源码，移植自 @applemusic-like-lyrics/core）。
 *
 * 直接把封面图喂给渲染器：缩样 + 高模糊 + 提艳后生成网格渐变，
 * 换封面时新旧网格按 alpha 交叉淡入，无需自行取色。
 * 替代原自研 PixiBackground（metaball 近似版），观感对齐 Apple Music。
 */
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { MeshGradientRenderer } from '../bg-render'
import { loadResourceFromUrl } from '../utils/resource'

const props = defineProps<{ cover?: string }>()

const canvas = ref<HTMLCanvasElement | null>(null)
let renderer: MeshGradientRenderer | null = null
// 每次换封面自增，避免异步竞态（旧图回来覆盖新封面）
let applyToken = 0
let profileToken = 0
let themeObserver: MutationObserver | null = null
let reducedMotionQuery: MediaQueryList | null = null
let displayRefreshHz: number | null = null
let displayRefreshProbe: Promise<number | null> | null = null
let gpuHealthy: 'unknown' | 'healthy' | 'degraded' | 'failed' = 'unknown'
let gpuStatusUnsubscribe: (() => void) | null = null

function currentThemeId(): string {
  return document.documentElement.dataset.theme ?? 'green'
}

/**
 * Probe the compositor once at startup. Screen has no standard refresh-rate
 * property, so a short finite RAF sample is the only portable hint available
 * while Electron is still the compatibility shell. The Rust profile remains
 * authoritative and clamps the result; this probe never runs continuously.
 */
function detectDisplayRefreshHz(): Promise<number | null> {
  if (displayRefreshHz !== null) return Promise.resolve(displayRefreshHz)
  if (displayRefreshProbe) return displayRefreshProbe
  displayRefreshProbe = new Promise((resolve) => {
    const samples: number[] = []
    let previous = 0
    let frame = 0
    const sample = (now: number): void => {
      if (previous > 0) samples.push(now - previous)
      previous = now
      frame++
      if (frame >= 8) {
        const usable = samples.filter((delta) => delta >= 4 && delta <= 100)
        if (usable.length === 0) {
          resolve(null)
          return
        }
        const average = usable.reduce((sum, delta) => sum + delta, 0) / usable.length
        displayRefreshHz = Math.max(1, Math.min(1_000, Math.round((1_000 / average) * 10) / 10))
        resolve(displayRefreshHz)
        return
      }
      window.requestAnimationFrame(sample)
    }
    window.requestAnimationFrame(sample)
  })
  return displayRefreshProbe
}

function applyMotionProfile(): void {
  const r = renderer
  if (!r) return
  const token = ++profileToken
  const reducedMotion = reducedMotionQuery?.matches ?? false
  const gpuUnavailable = gpuHealthy === 'degraded' || gpuHealthy === 'failed'
  void window.api.theme
    .motionProfile(currentThemeId(), reducedMotion, null, displayRefreshHz)
    .then((profile) => {
      if (token !== profileToken || renderer !== r) return
      // Native profile unavailable: keep the renderer's own scheduler default;
      // never impose a low fixed FPS from the compatibility layer.
      if (!profile) {
        r.setStaticMode(gpuUnavailable)
        return
      }
      r.setStaticMode(gpuUnavailable || !profile.enabled)
      if (profile.targetFps != null && profile.targetFps > 0) {
        r.setFPS(profile.targetFps)
      }
    })
    .catch(() => {
      if (token !== profileToken || renderer !== r) return
      r.setStaticMode(gpuUnavailable)
    })
}

function syncRendererActivity(): void {
  if (!renderer) return
  if (document.hidden || !document.hasFocus()) renderer.pause()
  else renderer.resume()
}

async function applyCover(src?: string): Promise<void> {
  const token = ++applyToken
  const r = renderer
  if (!r) return
  if (!src) {
    // 无封面：淡出所有网格状态
    await r.setAlbum(undefined)
    return
  }
  try {
    const img = await loadResourceFromUrl(src)
    if (token !== applyToken || renderer !== r) return
    await r.setAlbum(img)
  } catch (e) {
    if (token !== applyToken || renderer !== r) return
    console.warn('[bg-render] 封面载入失败', e)
    // 载入失败（缓存协议 502 等）：淡出到无封面，别让背景停在上一首的颜色上
    await r.setAlbum(undefined)
  }
}

onMounted(() => {
  if (!canvas.value) return
  const r = new MeshGradientRenderer(canvas.value)
  // 低渲染比例（0.3）+ CSS 过扫描（scale 1.3）：
  // BHP 网格 patch 交界在强色差下会露出多边形接缝（"弧线"），
  // 过扫描把边缘裁出屏幕、低分辨率渲染把接缝糊化，同时更省性能
  r.setRenderScale(0.3)
  // 没有音频频谱数据，给默认活跃度（AMLL 文档建议无数据时传 1.0）
  r.setLowFreqVolume(1)
  renderer = r
  const applyGpuStatus = (status: { healthy: typeof gpuHealthy }): void => {
    gpuHealthy = status.healthy
    applyMotionProfile()
  }
  gpuStatusUnsubscribe = window.api.app.onGpuStatus(applyGpuStatus)
  void window.api.app.gpuStatus().then(applyGpuStatus).catch(() => undefined)
  reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  reducedMotionQuery.addEventListener('change', applyMotionProfile)
  themeObserver = new MutationObserver((records) => {
    if (records.some((record) => record.attributeName === 'data-theme')) applyMotionProfile()
  })
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme']
  })
  applyMotionProfile()
  void detectDisplayRefreshHz().then((refreshHz) => {
    if (refreshHz !== null && renderer === r) applyMotionProfile()
  })
  window.addEventListener('blur', syncRendererActivity)
  window.addEventListener('focus', syncRendererActivity)
  document.addEventListener('visibilitychange', syncRendererActivity)
  syncRendererActivity()
  void applyCover(props.cover)
})

watch(
  () => props.cover,
  (c) => void applyCover(c)
)

onBeforeUnmount(() => {
  applyToken++
  profileToken++
  themeObserver?.disconnect()
  themeObserver = null
  reducedMotionQuery?.removeEventListener('change', applyMotionProfile)
  reducedMotionQuery = null
  gpuStatusUnsubscribe?.()
  gpuStatusUnsubscribe = null
  window.removeEventListener('blur', syncRendererActivity)
  window.removeEventListener('focus', syncRendererActivity)
  document.removeEventListener('visibilitychange', syncRendererActivity)
  renderer?.dispose()
  renderer = null
})
</script>

<template>
  <canvas ref="canvas" class="amll-bg" />
</template>

<style scoped>
.amll-bg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
  /* 过扫描：网格边缘与 patch 接缝裁出屏幕（配合 renderScale 0.3 糊化） */
  transform: scale(1.3);
}
</style>
