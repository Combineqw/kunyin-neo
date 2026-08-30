/**
 * 主窗口管理。
 */
import { app, BrowserWindow, Menu, shell } from 'electron'
import { appendFileSync } from 'node:fs'
import { join } from 'node:path'
import { is } from '@electron-toolkit/utils'
import { IpcChannels, WINDOW_SIZE_LIST } from '@common'
import { appEvent } from '../core/events'

const windowIcon = app.isPackaged
  ? join(process.resourcesPath, 'assets', 'icons', 'icon.png')
  : join(app.getAppPath(), 'resources', 'icons', 'icon.png')
import { getSettings } from '../store/settings'

let mainWindow: BrowserWindow | null = null
let isQuitting = false

function writeWindowDiagnostic(event: string, details: Record<string, unknown> = {}): void {
  const payload = JSON.stringify({
    timestamp: new Date().toISOString(),
    event,
    ...details
  })
  try {
    appendFileSync(join(app.getPath('userData'), 'window-diagnostics.log'), `${payload}\n`, 'utf8')
  } catch (error) {
    console.error('[main-window] failed to write diagnostic log', error)
  }
  console.error('[main-window]', event, details)
}

app.on('before-quit', () => {
  isQuitting = true
})

/**
 * 获取当前主窗口实例。
 *
 * @returns 主窗口；窗口尚未创建或已销毁时为 null。
 */
export function getMainWindow(): BrowserWindow | null {
  return mainWindow
}

/** 显示主窗口（幂等）。由渲染层 window:ready 触发，另有创建时的超时兜底。 */
export function showMainWindow(): void {
  if (!mainWindow || mainWindow.isDestroyed()) return
  if (mainWindow.isMinimized()) mainWindow.restore()
  if (!mainWindow.isVisible()) mainWindow.show()
  mainWindow.focus()
}

/**
 * 创建或聚焦主窗口。
 *
 * @returns 可用的主窗口实例；已有窗口会被直接复用。
 */
export function createMainWindow(): BrowserWindow {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.focus()
    return mainWindow
  }

  const settings = getSettings()
  // 按设置中的窗口尺寸档位创建（对应设置页"窗口尺寸"选项）
  const sizeConf =
    WINDOW_SIZE_LIST.find((i) => i.id === settings.appearance.windowSizeId) ?? WINDOW_SIZE_LIST[3]

  const win = new BrowserWindow({
    width: sizeConf.width,
    height: sizeConf.height,
    resizable: process.platform === 'linux' && settings.behavior.startInFullscreen,
    maximizable: false,
    fullscreenable: true,
    fullscreen: settings.behavior.startInFullscreen,
    show: false,
    autoHideMenuBar: true,
    title: '坤音neo',
    frame: false,
    // 关闭 Windows 11 DWM 的焦点色边框和外阴影；圆角由渲染层裁切，避免重新带回蓝边。
    roundedCorners: false,
    thickFrame: false,
    hasShadow: false,
    // 主窗口必须使用不透明背景：Windows 透明无框窗口在部分 DWM/GPU 环境中会显示为纯白，
    // 即使 renderer 已成功加载。桌面歌词和迷你播放器仍可独立使用透明窗口。
    transparent: false,
    backgroundColor: '#171918',
    // mac 的 Dock 图标由 app bundle 决定；win/linux 窗口图标显式给
    // （Windows 打包后 exe 图标接管，这里主要让 dev 模式任务栏不显示 Electron 默认图标）
    ...(process.platform !== 'darwin' ? { icon: windowIcon } : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      nodeIntegrationInWorker: true,
      contextIsolation: false,
      webSecurity: false,
      nodeIntegration: true,
      sandbox: false,
      enableWebSQL: false,
      spellcheck: false // 禁用拼写检查器
    }
  })

  Menu.setApplicationMenu(null)

  // 生产白屏排障：加载路径、渲染进程崩溃和 renderer console 错误必须进入主进程日志，
  // 否则无框窗口只会呈现空白，用户无法提供有效错误信息。
  win.webContents.on('dom-ready', () => {
    writeWindowDiagnostic('dom-ready', { url: win.webContents.getURL() })
  })
  win.webContents.on('did-finish-load', () => {
    writeWindowDiagnostic('did-finish-load', { url: win.webContents.getURL() })
  })
  win.webContents.on('did-fail-load', (_event, errorCode, errorDescription, validatedURL) => {
    writeWindowDiagnostic('did-fail-load', {
      errorCode,
      errorDescription,
      validatedURL
    })
  })
  win.webContents.on('render-process-gone', (_event, details) => {
    writeWindowDiagnostic('render-process-gone', details as unknown as Record<string, unknown>)
  })
  win.webContents.on('console-message', (_event, level, message, line, sourceId) => {
    if (level >= 2) {
      writeWindowDiagnostic('renderer-console', { level, message, line, sourceId })
    }
  })

  // 不在 ready-to-show 显示：此时首帧只有 #app 的主题背景图（Vue 尚未挂载），
  // 用户会看到背景图一闪而过。改等渲染层 window:ready（挂载 + 真实 UI 完成一帧绘制），
  // 超时兜底防止渲染异常导致窗口永不显示。
  const showFallback = setTimeout(showMainWindow, 8000)
  win.once('show', () => clearTimeout(showFallback))

  win.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  const notifyFullscreen = (): void => {
    if (!win.isDestroyed()) {
      win.webContents.send(IpcChannels.WINDOW_FULLSCREEN_CHANGED, win.isFullScreen())
    }
  }
  win.on('enter-full-screen', notifyFullscreen)
  win.on('leave-full-screen', notifyFullscreen)

  win.on('close', (event) => {
    if (isQuitting) return
    if (getSettings().behavior.closeToTray) {
      event.preventDefault()
      win.hide()
      return
    }
    // 桌面歌词窗口即使不可见也仍计入 BrowserWindow；只关闭主窗口会导致
    // window-all-closed 永远不触发。Windows/Linux 明确走 app.quit 关闭所有窗口。
    if (process.platform !== 'darwin') {
      event.preventDefault()
      app.quit()
    }
  })

  // HMR：开发期加载 dev server，生产加载打包后的 index.html
  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    const rendererUrl = process.env['ELECTRON_RENDERER_URL']
    writeWindowDiagnostic('load-url', { rendererUrl })
    // 修复: BrowserWindow 加载失败会 reject；记录它可避免主进程出现未处理 Promise 拒绝。
    void win.loadURL(rendererUrl).catch((error) => {
      writeWindowDiagnostic('load-url-rejected', { error: String(error) })
    })
  } else {
    const rendererPath = join(__dirname, '../renderer/index.html')
    writeWindowDiagnostic('load-file', { rendererPath, packaged: app.isPackaged })
    // 修复: 生产 HTML 加载失败同样需要消化 Promise 拒绝，did-fail-load 负责补充页面级诊断。
    void win.loadFile(rendererPath).catch((error) => {
      writeWindowDiagnostic('load-file-rejected', { error: String(error) })
    })
  }

  win.on('closed', () => {
    // 修复: 窗口在首帧前关闭时，释放尚未触发的显示兜底计时器。
    clearTimeout(showFallback)
    mainWindow = null
  })

  mainWindow = win
  appEvent.emit('main-window-created', win)
  return win
}
