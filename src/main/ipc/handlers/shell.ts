/**
 * 系统对话框 / Shell IPC（下载路径选择、打开目录等）。
 */
import { dialog, shell } from 'electron'
import { IpcChannels } from '@common'
import { handle } from '../helpers'
import { getMainWindow } from '../../windows/main'

export function registerShellHandlers(): void {
  handle(IpcChannels.DIALOG_SELECT_DIRECTORY, async (defaultPath?: string) => {
    const win = getMainWindow()
    const r = await dialog.showOpenDialog(win ?? undefined!, {
      defaultPath: defaultPath || undefined,
      properties: ['openDirectory', 'createDirectory']
    })
    return r.canceled || !r.filePaths.length ? null : r.filePaths[0]
  })

  handle(
    IpcChannels.DIALOG_SELECT_OPEN_FILE,
    (filters: { name: string; extensions: string[] }[]) => {
      const win = getMainWindow()
      return dialog.showOpenDialog(win ?? undefined!, {
        properties: ['openFile'],
        filters
      }).then((r) => (r.canceled || !r.filePaths.length ? null : r.filePaths[0]))
    }
  )

  handle(
    IpcChannels.DIALOG_SELECT_SAVE_FILE,
    (defaultName: string, filters: { name: string; extensions: string[] }[]) => {
      const win = getMainWindow()
      return dialog.showSaveDialog(win ?? undefined!, {
        defaultPath: defaultName,
        filters
      }).then((r) => (r.canceled || !r.filePath ? null : r.filePath))
    }
  )

  handle(IpcChannels.SHELL_OPEN_PATH, (path: string) => shell.openPath(path))
}
