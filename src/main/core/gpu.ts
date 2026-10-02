/**
 * Chromium GPU health monitor.
 *
 * GPU command-line switches are intentionally not set here.  Disabling GPU at
 * process start forces every WebGL/compositor surface onto software rendering
 * and makes an otherwise healthy machine pay the CPU cost permanently.  This
 * monitor only observes the runtime and lets the renderer choose a static
 * fallback after a GPU process failure.
 */
import { app } from 'electron'
import { IpcChannels, type GpuRuntimeStatus } from '@common'
import { sendToAllRenderers } from '../ipc/helpers'

let installed = false
let featureStatusReady = false
let gpuProcessCrashes = 0
let currentStatus: GpuRuntimeStatus = {
  accelerated: null,
  healthy: 'unknown',
  features: {},
  gpuProcessCrashes,
  failureReason: null,
  updatedAt: 0
}

function readFeatureStatus(): Record<string, string> {
  if (!featureStatusReady) return {}
  try {
    return { ...app.getGPUFeatureStatus() }
  } catch {
    return {}
  }
}

function featureEnabled(value: string | undefined): boolean {
  // Electron/Chromium report values such as `enabled`, `disabled_off`,
  // `unavailable_software`, and `unknown`; only the explicit enabled forms
  // should allow animated GPU surfaces.
  return value === 'enabled' || value === 'force_enabled'
}

function refreshStatus(failureReason: string | null = null): void {
  const features = readFeatureStatus()
  let accelerated: boolean | null = null
  if (featureStatusReady) {
    try {
      accelerated = app.isHardwareAccelerationEnabled()
    } catch {
      accelerated = null
    }
  }

  const compositor = features.gpu_compositing
  const webgl = features.webgl
  const healthy = failureReason
    ? 'failed'
    : accelerated === null
      ? 'unknown'
      : accelerated && featureEnabled(compositor) && featureEnabled(webgl)
        ? 'healthy'
        : 'degraded'

  currentStatus = {
    accelerated,
    healthy,
    features,
    gpuProcessCrashes,
    failureReason,
    updatedAt: Date.now()
  }
  // A crash can happen after the renderer has mounted.  Push a small status
  // object so animated surfaces can enter static mode without a polling loop.
  sendToAllRenderers(IpcChannels.GPU_STATUS_CHANGED, currentStatus)
}

/** Register once, before app ready, so the first gpu-info-update is observed. */
export function installGpuMonitor(): void {
  if (installed) return
  installed = true

  app.on('gpu-info-update', () => {
    featureStatusReady = true
    refreshStatus(null)
  })
  app.on('child-process-gone', (_event, details) => {
    if (details.type !== 'GPU') return
    gpuProcessCrashes += 1
    refreshStatus(`GPU process ${details.reason}`)
  })

  if (app.isReady()) {
    featureStatusReady = true
    refreshStatus(null)
  }
}

export function getGpuStatus(): GpuRuntimeStatus {
  return { ...currentStatus, features: { ...currentStatus.features } }
}
