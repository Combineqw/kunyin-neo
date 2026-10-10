/**
 * Tauri 2 原生 Rust 桥接适配器。
 * 当在 Tauri 运行时（如 Edge WebView2 原生宿主）下执行时，将 window.api 代理至 Rust 命令通道；
 * 在 Electron 运行时下自动无操作透传，零性能损耗与零回归破坏。
 */
import type { WindowApi } from '@common'

export function isTauriEnvironment(): boolean {
  if (typeof window === 'undefined') return false
  return (
    '__TAURI_INTERNALS__' in window ||
    '__TAURI__' in window ||
    Boolean((window as unknown as { isTauri?: boolean }).isTauri)
  )
}

// 帮助函数：安全调用 Tauri 核心 invoke
export async function tauriInvoke<T = unknown>(
  cmd: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const tauriObj = (
    window as unknown as {
      __TAURI__?: {
        core?: { invoke: (c: string, a: Record<string, unknown>) => Promise<T> }
        invoke?: (c: string, a: Record<string, unknown>) => Promise<T>
      }
    }
  ).__TAURI__
  if (tauriObj?.core?.invoke) {
    return tauriObj.core.invoke(cmd, args)
  }
  if (tauriObj?.invoke) {
    return tauriObj.invoke(cmd, args)
  }
  throw new Error(`[TauriBridge] 未检测到 Tauri invoke 通道: ${cmd}`)
}

export function createTauriApi(): WindowApi {
  const handler: ProxyHandler<object> = {
    get(_target, prop: string | symbol) {
      if (typeof prop !== 'string') return undefined

      return new Proxy(
        {},
        {
          get(_subTarget, method: string | symbol) {
            if (typeof method !== 'string') return undefined

            // 事件监听与订阅函数返回统一的取消订阅闭包
            if (method.startsWith('on') || method === 'onChange') {
              return () => () => {}
            }

            // 同步与特定返回值函数
            if (prop === 'settings' && method === 'setSync') {
              return (patch: unknown) => patch
            }

            // 异步方法调用代理
            return async (...args: unknown[]) => {
              if (prop === 'app' && method === 'getPlatform') {
                return 'win32'
              }
              if (prop === 'app' && method === 'getVersion') {
                return tauriInvoke<string>('app_version').catch(() => '1.1.0')
              }
              if (prop === 'settings' && method === 'get') {
                return tauriInvoke('read_settings').catch(() => ({}))
              }
              if (prop === 'settings' && method === 'set') {
                return tauriInvoke('update_settings', { patch: args[0] }).catch(
                  () => args[0] ?? {}
                )
              }
              if (prop === 'settings' && method === 'proxyStatus') {
                return { enabled: false, active: true, url: '', message: 'direct' }
              }
              if (prop === 'window' && method === 'minimize') {
                return tauriInvoke('window_minimize').catch(() => {})
              }
              if (prop === 'window' && method === 'close') {
                return tauriInvoke('window_close').catch(() => {})
              }
              if (prop === 'window' && method === 'fullscreen') {
                return tauriInvoke('window_toggle_maximize').catch(() => false)
              }
              if (prop === 'library' && method === 'playlists') {
                return tauriInvoke('library_list_playlists').catch(() => [])
              }
              if (prop === 'library' && method === 'playlistSongs') {
                return tauriInvoke('library_query_songs', { playlistId: args[0] }).catch(() => [])
              }
              if (prop === 'library' && method === 'createPlaylist') {
                const opts = args[1] as
                  | { remoteSource?: string; remoteId?: string; autoRefresh?: boolean }
                  | undefined
                return tauriInvoke('library_create_playlist', {
                  name: args[0],
                  remoteSource: opts?.remoteSource ?? null,
                  remoteId: opts?.remoteId ?? null,
                  autoRefresh: opts?.autoRefresh ?? false
                }).catch(() => 0)
              }

              // 通用指令兜底映射
              const cmdName = `${prop}_${method}`
              return tauriInvoke(cmdName, { args }).catch((err) => {
                console.warn(`[TauriBridge] ${prop}.${method} unhandled in Tauri runtime:`, err)
                return null
              })
            }
          }
        }
      )
    }
  }

  return new Proxy({}, handler) as WindowApi
}

export function initTauriBridge(): void {
  if (typeof window === 'undefined') return
  if (!isTauriEnvironment()) return
  if ((window as unknown as { api?: WindowApi }).api) return

  ;(window as unknown as { api: WindowApi }).api = createTauriApi()
  console.info('[TauriBridge] 成功注入 Tauri 2 原生 Rust WindowApi 桥接层')
}
