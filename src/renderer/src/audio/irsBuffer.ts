/**
 * IRS 持久化数据与 Web Audio 解码输入之间的无状态转换。
 * 预设加载竞争控制和 ConvolverNode 状态仍由 player store 与 audioGraph 分别负责。
 */

/**
 * 将持久化的 Base64 IRS 数据还原为 ArrayBuffer。
 * @param value 设置中保存的 Base64 音频内容。
 * @returns 可传入 AudioContext.decodeAudioData 的独立字节缓冲区。
 */
export function base64ToArrayBuffer(value: string): ArrayBuffer {
  const binary = atob(value)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index)
  return bytes.buffer
}
