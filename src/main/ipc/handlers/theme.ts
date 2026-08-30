// 主题文件导入导出。与 handlers/animation.ts 同构：主进程负责文件对话框与磁盘读写，
// 校验统一走 @common 的 themeConfigFromJson（安全红线的唯一入口）。
import { dialog } from 'electron'
import { readFile, writeFile } from 'node:fs/promises'
import { basename } from 'node:path'
import {
  IpcChannels,
  themeConfigFromJson,
  themeConfigToJson,
  themeFileName,
  type ThemeFileConfig
} from '@common'
import { handle } from '../helpers'
import { getMainWindow } from '../../windows/main'

const filters = [{ name: '坤音neo 主题', extensions: ['json', 'kytheme'] }]

export function registerThemeHandlers(): void {
  handle(IpcChannels.THEME_IMPORT_FILE, async (): Promise<ThemeFileConfig | null> => {
    const result = await dialog.showOpenDialog(getMainWindow() ?? undefined!, {
      title: '导入主题',
      properties: ['openFile'],
      filters
    })
    if (result.canceled || !result.filePaths[0]) return null
    const text = await readFile(result.filePaths[0], 'utf8')
    let raw: unknown
    try {
      raw = JSON.parse(text)
    } catch {
      // JSON.parse 的原生报错含行列号，对用户无意义，换成可读文案
      throw new Error('主题文件不是合法的 JSON（文件可能已损坏）')
    }
    return themeConfigFromJson(raw)
  })

  handle(IpcChannels.THEME_EXPORT_FILE, async (value: ThemeFileConfig): Promise<string | null> => {
    // 先校验再落盘：保证导出的文件一定可被导入
    const safe = themeConfigToJson(value)
    const result = await dialog.showSaveDialog(getMainWindow() ?? undefined!, {
      title: '导出主题',
      defaultPath: themeFileName(safe.name),
      filters
    })
    if (result.canceled || !result.filePath) return null
    await writeFile(result.filePath, `${JSON.stringify(safe, null, 2)}\n`, 'utf8')
    return basename(result.filePath)
  })
}
