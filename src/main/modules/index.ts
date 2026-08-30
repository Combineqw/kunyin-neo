/**
 * 主进程常驻模块注册器（参照 lx-music-desktop 的 registerModules）。
 *
 * 已接入：
 * - media：全局媒体键、托盘、开机自启（SMTC 显示走渲染层 Web MediaSession）
 * - sync：LX Music 同步客户端（Phase 5.2）
 * - backup：备份/导入（Phase 5.3）
 * - updater：自动更新（Phase 5.3）
 * 说明：下载队列走 IPC handler 按需初始化；音频代理走 audio/protocol（app ready 时装）。
 */
import { registerMediaModule } from './media'
import { registerDesktopLyricModule } from './desktop-lyrics/window'
import { registerMiniPlayerModule } from './mini-player/window'
import { registerSyncModule } from './sync'
import { registerBackupHandlers } from './backup'
import { registerUpdaterModule } from './updater'

/**
 * 注册依赖主进程常驻生命周期的功能模块。
 *
 * @returns 无返回值；模块各自注册事件和 IPC 监听器。
 */
export function registerModules(): void {
  registerMediaModule()
  registerDesktopLyricModule()
  registerMiniPlayerModule()
  registerSyncModule()
  registerBackupHandlers()
  registerUpdaterModule()
}
