/**
 * 本地歌曲导入（LX「添加本地歌曲」）。
 *
 * 弹系统文件选择框（多选音频文件）→ music-metadata 解析标签 → LocalMusicItem 入歌单。
 * - id 为文件绝对路径的 SHA-1 前 52 bit（确定性数值，重复导入被 INSERT OR IGNORE 去重）；
 * - 标签缺失时回退文件名解析（"艺术家 - 标题" 或纯标题）；
 * - 内嵌封面不入库（song_json 会进 SQLite，塞 base64 图会把库撑爆），封面留空。
 */
import { dialog } from 'electron'
import { getMainWindow } from '../../windows/main'
import { AUDIO_EXTENSIONS } from './core'

export { AUDIO_EXTENSIONS, localSongId, parseFileName, parseLocalSong } from './core'

/** 弹多选文件框；取消返回 null */
export async function pickLocalSongs(): Promise<string[] | null> {
  const win = getMainWindow()
  const r = await dialog.showOpenDialog(win ?? undefined!, {
    title: '添加本地歌曲',
    properties: ['openFile', 'multiSelections'],
    filters: [{ name: '音频文件', extensions: AUDIO_EXTENSIONS }]
  })
  return r.canceled || !r.filePaths.length ? null : r.filePaths
}
