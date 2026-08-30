/**
 * 播放层共享的音质降级阶梯。
 * 实际可用性、AI 音质屏蔽和直链请求仍由 player store 根据歌曲与设置计算。
 */
import type { QualityId } from '@common'

/**
 * 从低到高的默认音质候选顺序。
 * @returns 供播放 store 迭代取流的稳定音质标识列表。
 */
export const QUALITY_LADDER: QualityId[] = [
  '128k',
  '320k',
  'flac',
  'hires',
  'master',
  'atmos',
  'atmos_plus'
]
