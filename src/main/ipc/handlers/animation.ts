import { dialog } from 'electron'
import { readFile, writeFile } from 'node:fs/promises'
import { basename } from 'node:path'
import { animationPackFromJson, animationPackToJson, IpcChannels, type AnimationPackJson } from '@common'
import { handle } from '../helpers'
import { getMainWindow } from '../../windows/main'

const filters = [{ name: '坤音neo 动效包', extensions: ['json', 'kyanim'] }]

export function registerAnimationHandlers(): void {
  handle(IpcChannels.ANIMATION_IMPORT_FILE, async (): Promise<AnimationPackJson | null> => {
    const result = await dialog.showOpenDialog(getMainWindow() ?? undefined!, {
      title: '导入动效包',
      properties: ['openFile'],
      filters
    })
    if (result.canceled || !result.filePaths[0]) return null
    const raw = JSON.parse(await readFile(result.filePaths[0], 'utf8')) as unknown
    return animationPackToJson(animationPackFromJson(raw))
  })

  handle(
    IpcChannels.ANIMATION_EXPORT_FILE,
    async (value: AnimationPackJson): Promise<string | null> => {
      const safe = animationPackToJson(animationPackFromJson(value))
      const result = await dialog.showSaveDialog(getMainWindow() ?? undefined!, {
        title: '导出动效包',
        defaultPath: `${safe.name.replace(/[\\/:*?"<>|]/g, '_')}.kyanim.json`,
        filters
      })
      if (result.canceled || !result.filePath) return null
      await writeFile(result.filePath, `${JSON.stringify(safe, null, 2)}\n`, 'utf8')
      return basename(result.filePath)
    }
  )
}
