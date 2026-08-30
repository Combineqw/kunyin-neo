/**
 * 均衡器配置导入/导出 IPC。
 * 格式识别和文本映射委托 common/domain 纯函数；本模块只处理原生文件对话框与磁盘读写。
 */
import { dialog } from 'electron'
import { readFile, writeFile } from 'node:fs/promises'
import { basename, extname } from 'node:path'
import {
  EQUALIZER_PROFILE_FORMAT,
  EQUALIZER_PROFILE_VERSION,
  IpcChannels,
  normalizeEqualizerProfile,
  parseExternalEqualizerText,
  serializeEqualizerApoText,
  type EqualizerProfile,
  type EqualizerProfileImportResult
} from '@common'
import { handle } from '../helpers'
import { getMainWindow } from '../../windows/main'

const importFilters = [
  { name: '调音文件', extensions: ['kyeq', 'json', 'txt', 'cfg', 'apo'] },
  { name: '坤音neo 调音文件', extensions: ['kyeq', 'json'] },
  { name: 'Equalizer APO / AutoEq 文本', extensions: ['txt', 'cfg', 'apo'] }
]

const exportFilters = [
  { name: 'Equalizer APO 调音文件', extensions: ['txt'] },
  { name: '坤音neo 调音文件', extensions: ['kyeq', 'json'] }
]

function readProfile(text: string, filePath: string): EqualizerProfile {
  const extension = extname(filePath).toLowerCase()
  if (extension === '.kyeq' || extension === '.json') {
    try {
      return normalizeEqualizerProfile(JSON.parse(text))
    } catch (error) {
      const message = error instanceof Error ? error.message : '未知 JSON 错误'
      throw new Error(`无法读取 JSON 调音文件：${message}`)
    }
  }
  try {
    return parseExternalEqualizerText(text, basename(filePath))
  } catch (error) {
    const message = error instanceof Error ? error.message : '未知文本格式错误'
    throw new Error(`无法解析 Equalizer APO / AutoEq 文本：${message}`)
  }
}

/**
 * 注册 EQ 配置文件导入与导出通道。
 *
 * @returns 无返回值；文件选择和序列化结果由各 IPC 请求返回。
 */
export function registerEqualizerHandlers(): void {
  handle(IpcChannels.EQ_IMPORT_FILE, async (): Promise<EqualizerProfileImportResult | null> => {
    const win = getMainWindow()
    const result = await dialog.showOpenDialog(win ?? undefined!, {
      title: '导入调音文件',
      properties: ['openFile'],
      filters: importFilters
    })
    if (result.canceled || !result.filePaths[0]) return null

    const filePath = result.filePaths[0]
    const text = await readFile(filePath, 'utf8')
    return { profile: readProfile(text, filePath), filePath }
  })

  handle(IpcChannels.EQ_EXPORT_FILE, async (value: EqualizerProfile): Promise<string | null> => {
    const profile = normalizeEqualizerProfile(value)
    const win = getMainWindow()
    const defaultName = `${profile.name.replace(/[\\/:*?"<>|]/g, '_')}.txt`
    const result = await dialog.showSaveDialog(win ?? undefined!, {
      title: '导出调音文件',
      defaultPath: defaultName,
      filters: exportFilters
    })
    if (result.canceled || !result.filePath) return null

    const extension = extname(result.filePath).toLowerCase()
    if (extension === '.kyeq' || extension === '.json') {
      const output: EqualizerProfile = {
        ...profile,
        format: EQUALIZER_PROFILE_FORMAT,
        version: EQUALIZER_PROFILE_VERSION,
        source: profile.source ?? 'local'
      }
      await writeFile(result.filePath, `${JSON.stringify(output, null, 2)}\n`, 'utf8')
    } else {
      await writeFile(result.filePath, serializeEqualizerApoText(profile), 'utf8')
    }
    return basename(result.filePath)
  })
}
