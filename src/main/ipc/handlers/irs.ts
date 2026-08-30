/**
 * IRS 文件导入 IPC。
 * 仅负责选择、读取和校验 RIFF/WAVE 元数据，并返回可持久化的预设快照；AudioBuffer 解码始终在渲染层音频图完成。
 */
import { dialog } from 'electron'
import { basename, extname } from 'node:path'
import { readFile } from 'node:fs/promises'
import { randomUUID } from 'node:crypto'
import {
  IpcChannels,
  IRS_PROFILE_FORMAT,
  IRS_PROFILE_VERSION,
  readIrsWaveInfo,
  type IrsProfileImportResult
} from '@common'
import { handle } from '../helpers'
import { getMainWindow } from '../../windows/main'

const importFilters = [
  { name: '脉冲响应文件', extensions: ['irs', 'wav'] },
  { name: '所有文件', extensions: ['*'] }
]

function bytesToBase64(bytes: Uint8Array): string {
  return Buffer.from(bytes).toString('base64')
}

/**
 * 注册 IRS 导入通道。
 *
 * @returns 无返回值；注册结果由 IPC handler 生命周期持有。
 */
export function registerIrsHandlers(): void {
  handle(IpcChannels.IRS_IMPORT_FILE, async (): Promise<IrsProfileImportResult | null> => {
    const win = getMainWindow()
    const result = await dialog.showOpenDialog(win ?? undefined!, {
      title: '导入 IRS 脉冲响应',
      properties: ['openFile'],
      filters: importFilters
    })
    if (result.canceled || !result.filePaths[0]) return null

    const filePath = result.filePaths[0]
    const extension = extname(filePath).toLowerCase()
    if (extension !== '.irs' && extension !== '.wav') {
      throw new Error('请选择 .irs 或 .wav 脉冲响应文件')
    }
    const bytes = new Uint8Array(await readFile(filePath))
    const info = readIrsWaveInfo(bytes)
    const fileName = basename(filePath)
    const profile = {
      format: IRS_PROFILE_FORMAT,
      version: IRS_PROFILE_VERSION,
      id: randomUUID(),
      name: fileName.replace(/\.(?:irs|wav)$/i, ''),
      fileName,
      dataBase64: bytesToBase64(bytes),
      durationMs: Math.round(info.durationMs),
      sampleRate: info.sampleRate,
      channels: info.channels,
      source: 'imported' as const
    }
    return { profile, filePath }
  })
}
