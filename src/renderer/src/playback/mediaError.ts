/**
 * 播放层的无状态错误文案映射。
 * 只把浏览器 MediaError 转成可显示文本，不读取 Pinia 状态也不触发播放控制。
 */

/**
 * 将 HTMLMediaElement 的错误对象转换为面向用户的播放失败提示。
 * @param error 浏览器媒体元素报告的错误；允许 null 以覆盖未知失败。
 * @returns 可直接在播放器 UI 中展示的中文错误说明。
 */
export function describeMediaError(error: MediaError | null): string {
  switch (error?.code) {
    case MediaError.MEDIA_ERR_NETWORK:
      return '网络错误，播放中断（若开了代理请检查代理是否可用）'
    case MediaError.MEDIA_ERR_DECODE:
      return '音频解码失败'
    case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
      return '无法载入音频（直链失效或代理不可用）'
    case MediaError.MEDIA_ERR_ABORTED:
      return '播放已取消'
    default:
      return '播放失败'
  }
}
