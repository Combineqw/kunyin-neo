type SyncCallback = () => void

const callbacks = new Set<SyncCallback>()
let timer: ReturnType<typeof setInterval> | null = null

function stopIfIdle(): void {
  if (callbacks.size === 0 && timer) {
    clearInterval(timer)
    timer = null
  }
}

function startIfNeeded(): void {
  if (timer || callbacks.size === 0) return
  timer = setInterval(() => {
    for (const callback of callbacks) callback()
  }, 100)
}

/** Share one 100ms renderer tick between lightweight cross-window state bridges. */
export function subscribeRuntimeSync(callback: SyncCallback): () => void {
  callbacks.add(callback)
  startIfNeeded()
  return () => {
    callbacks.delete(callback)
    stopIfIdle()
  }
}
