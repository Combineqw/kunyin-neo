/**
 * IRS 脉冲响应文件的轻量 RIFF/WAVE 元数据读取器。
 * 这里故意不解码样本：主进程仅需要安全校验和时长信息，实际 AudioBuffer 解码由渲染层 Web Audio 图完成。
 */
export interface IrsWaveInfo {
  durationMs: number
  sampleRate: number
  channels: number
  bitsPerSample: number
  audioFormat: number
  dataBytes: number
}

const RIFF = 0x52494646
const WAVE = 0x57415645
const FMT = 0x666d7420
const DATA = 0x64617461

/**
 * 读取 IRS 常见 RIFF/WAVE 头信息，不解码音频样本。
 * @param data 待校验的完整 IRS 文件字节。
 * @returns 供导入提示和持久化使用的音频元数据。
 * @throws 文件头、chunk 边界或关键音频字段无效时抛出错误。
 */
export function readIrsWaveInfo(data: Uint8Array): IrsWaveInfo {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength)
  if (
    data.byteLength < 12 ||
    view.getUint32(0, false) !== RIFF ||
    view.getUint32(8, false) !== WAVE
  ) {
    throw new Error('文件不是有效的 RIFF/WAVE 脉冲响应文件')
  }

  let offset = 12
  let audioFormat = 0
  let channels = 0
  let sampleRate = 0
  let byteRate = 0
  let bitsPerSample = 0
  let dataBytes = 0
  let foundFmt = false
  let foundData = false

  while (offset + 8 <= data.byteLength) {
    const chunkId = view.getUint32(offset, false)
    const chunkSize = view.getUint32(offset + 4, true)
    const chunkStart = offset + 8
    if (chunkStart > data.byteLength || chunkSize > data.byteLength - chunkStart) {
      throw new Error('RIFF/WAVE chunk 长度超出文件范围')
    }

    if (chunkId === FMT && chunkSize >= 16) {
      audioFormat = view.getUint16(chunkStart, true)
      channels = view.getUint16(chunkStart + 2, true)
      sampleRate = view.getUint32(chunkStart + 4, true)
      byteRate = view.getUint32(chunkStart + 8, true)
      bitsPerSample = view.getUint16(chunkStart + 14, true)
      foundFmt = true
    } else if (chunkId === DATA) {
      dataBytes = chunkSize
      foundData = true
    }

    offset = chunkStart + chunkSize + (chunkSize % 2)
  }

  if (!foundFmt || !foundData || !sampleRate || !channels || !byteRate || !dataBytes) {
    throw new Error('IRS 文件缺少有效音频数据')
  }
  if (!bitsPerSample || !audioFormat) {
    throw new Error('IRS 文件缺少有效音频格式信息')
  }

  return {
    durationMs: (dataBytes / byteRate) * 1000,
    sampleRate,
    channels,
    bitsPerSample,
    audioFormat,
    dataBytes
  }
}
