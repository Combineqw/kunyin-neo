import { BrowserWindow, ipcMain, screen, type Rectangle } from 'electron'
import { join } from 'node:path'
import { is } from '@electron-toolkit/utils'
import {
  IpcChannels,
  type AppSettings,
  type MiniPlayerCommand,
  type MiniPlayerState
} from '@common'
import { appEvent } from '../../core/events'
import { sendToAllRenderers } from '../../ipc/helpers'
import { getSettings, updateSettings } from '../../store/settings'
import { getMainWindow } from '../../windows/main'

let miniPlayerWindow: BrowserWindow | null = null
let lastState: MiniPlayerState | null = null
let saveBoundsTimer: ReturnType<typeof setTimeout> | null = null

const COLLAPSED_SIZE = { width: 72, height: 72 }
const EXPANDED_SIZE = { width: 348, height: 92 }

function keepInDisplay(bounds: Rectangle): Rectangle {
  const area = screen.getDisplayMatching(bounds).workArea
  const width = Math.min(bounds.width, area.width)
  const height = Math.min(bounds.height, area.height)
  return {
    x: Math.min(Math.max(bounds.x, area.x), area.x + area.width - width),
    y: Math.min(Math.max(bounds.y, area.y), area.y + area.height - height),
    width,
    height
  }
}

function initialBounds(settings: AppSettings): Rectangle {
  const area = screen.getPrimaryDisplay().workArea
  const { miniPlayerX, miniPlayerY } = settings.player
  const x = miniPlayerX ?? area.x + area.width - EXPANDED_SIZE.width - 28
  const y = miniPlayerY ?? area.y + area.height - EXPANDED_SIZE.height - 104
  return keepInDisplay({ x, y, ...EXPANDED_SIZE })
}

function persistPosition(): void {
  const win = miniPlayerWindow
  if (!win || win.isDestroyed()) return
  if (saveBoundsTimer) clearTimeout(saveBoundsTimer)
  saveBoundsTimer = setTimeout(() => {
    if (!miniPlayerWindow || miniPlayerWindow.isDestroyed()) return
    const { x, y } = miniPlayerWindow.getBounds()
    updateSettings({ player: { miniPlayerX: x, miniPlayerY: y } })
  }, 350)
}

function createMiniPlayerWindow(): BrowserWindow {
  if (miniPlayerWindow && !miniPlayerWindow.isDestroyed()) return miniPlayerWindow

  const bounds = initialBounds(getSettings())
  const win = new BrowserWindow({
    ...bounds,
    minWidth: COLLAPSED_SIZE.width,
    minHeight: COLLAPSED_SIZE.height,
    maxWidth: EXPANDED_SIZE.width,
    maxHeight: EXPANDED_SIZE.height,
    useContentSize: true,
    frame: false,
    // Linux 缺少合成器时透明窗口不可靠，维持悬浮控制但使用不透明深色底降级。
    transparent: process.platform !== 'linux',
    backgroundColor: process.platform === 'linux' ? '#1f211f' : '#00000000',
    resizable: false,
    minimizable: false,
    maximizable: false,
    fullscreenable: false,
    roundedCorners: false,
    skipTaskbar: true,
    alwaysOnTop: true,
    hasShadow: false,
    show: false,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
      backgroundThrottling: false
    }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    // 修复: 独立窗口加载失败会 reject；显式记录以避免未处理 Promise 拒绝。
    void win.loadURL(`${process.env['ELECTRON_RENDERER_URL']}/mini-player.html`).catch((error) => {
      console.error('[mini-player] loadURL rejected', error)
    })
  } else {
    // 修复: 打包后的迷你播放器入口加载失败也必须被消费，防止主进程静默拒绝。
    void win.loadFile(join(__dirname, '../renderer/mini-player.html')).catch((error) => {
      console.error('[mini-player] loadFile rejected', error)
    })
  }

  win.once('ready-to-show', () => {
    win.showInactive()
    if (lastState) win.webContents.send(IpcChannels.MINI_PLAYER_STATE, lastState)
  })
  win.on('move', persistPosition)
  win.on('closed', () => {
    if (saveBoundsTimer) clearTimeout(saveBoundsTimer)
    saveBoundsTimer = null
    miniPlayerWindow = null
  })

  miniPlayerWindow = win
  return win
}

function showMiniPlayer(): void {
  const win = createMiniPlayerWindow()
  if (win.isMinimized()) win.restore()
  if (!win.isVisible()) win.showInactive()
}

function hideMiniPlayer(): void {
  if (miniPlayerWindow && !miniPlayerWindow.isDestroyed()) miniPlayerWindow.close()
  miniPlayerWindow = null
}

function syncMiniPlayerWindow(settings = getSettings()): void {
  if (settings.player.miniPlayerEnabled) showMiniPlayer()
  else hideMiniPlayer()
}

function commitPlayerSettings(patch: Partial<AppSettings['player']>): void {
  const next = updateSettings({ player: patch })
  sendToAllRenderers(IpcChannels.SETTINGS_CHANGED, next)
  appEvent.emit('settings-updated', next)
}

function sendCommand(command: MiniPlayerCommand): void {
  const mainWindow = getMainWindow()
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send(IpcChannels.MEDIA_COMMAND, command)
  }
}

function setExpanded(expanded: boolean): void {
  const win = miniPlayerWindow
  if (!win || win.isDestroyed()) return
  const bounds = win.getBounds()
  const size = expanded ? EXPANDED_SIZE : COLLAPSED_SIZE
  win.setBounds(keepInDisplay({ x: bounds.x, y: bounds.y, ...size }))
}

/**
 * 注册迷你播放器窗口的 IPC、设置同步和主窗口生命周期处理。
 *
 * @returns 无返回值。
 */
export function registerMiniPlayerModule(): void {
  ipcMain.handle(IpcChannels.MINI_PLAYER_TOGGLE, (_event, enabled: boolean) => {
    commitPlayerSettings({ miniPlayerEnabled: enabled })
  })

  ipcMain.on(IpcChannels.MINI_PLAYER_PUSH, (_event, state: MiniPlayerState) => {
    lastState = state
    if (miniPlayerWindow && !miniPlayerWindow.isDestroyed()) {
      miniPlayerWindow.webContents.send(IpcChannels.MINI_PLAYER_STATE, state)
    }
  })

  ipcMain.on(IpcChannels.MINI_PLAYER_COMMAND, (_event, command: MiniPlayerCommand) => {
    sendCommand(command)
  })

  ipcMain.on(IpcChannels.MINI_PLAYER_SET_EXPANDED, (_event, expanded: boolean) => {
    setExpanded(!!expanded)
  })

  appEvent.on('settings-updated', syncMiniPlayerWindow)
  appEvent.on('main-window-created', (win) => {
    win.on('closed', hideMiniPlayer)
  })
  appEvent.on('app-inited', () => syncMiniPlayerWindow())
}
