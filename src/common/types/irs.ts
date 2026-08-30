export const IRS_PROFILE_FORMAT = 'kunyin-irs' as const
export const IRS_PROFILE_VERSION = 1 as const

/** 已导入的脉冲响应预设。音频数据以 Base64 保存，避免依赖原始文件路径。 */
export interface IrsProfile {
  format: typeof IRS_PROFILE_FORMAT
  version: typeof IRS_PROFILE_VERSION
  id: string
  name: string
  fileName: string
  dataBase64: string
  durationMs: number
  sampleRate: number
  channels: number
  source: 'local' | 'imported'
}

export interface IrsProfileImportResult {
  profile: IrsProfile
  filePath: string
}
