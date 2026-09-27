/**
 * 本地曲库 IPC —— 分发到 store/library.ts。
 * 写操作后由 library 的 onLibraryChange 回调广播 LIBRARY_CHANGED，渲染层重拉。
 */
import {
  IpcChannels,
  type LibraryScanProgress,
  type LocalMusicItem,
  type LocalPlaylist,
  type MusicItem,
  type PlaylistSortField,
  type PlaylistSortOrder
} from '@common'
import { randomUUID } from 'node:crypto'
import { handle, sendToRenderer } from '../helpers'
import * as library from '../../store/library'
import {
  parseLocalSong,
  pickLocalDirectory,
  pickLocalSongs,
  scanLocalSongsProgressive,
  type LocalScanProgress
} from '../../modules/local-music'
import { enrichLocalSongs } from '../../modules/local-music/enrich'

let wired = false
const activeScans = new Map<string, { playlistId: number; controller: AbortController }>()
const activeEnrichments = new Map<string, { playlistId: number; controller: AbortController }>()

function activeTaskId(): string | undefined {
  return activeScans.keys().next().value ?? activeEnrichments.keys().next().value
}

function emitScanProgress(progress: LibraryScanProgress): void {
  sendToRenderer(IpcChannels.LIBRARY_SCAN_PROGRESS, progress)
}

export function registerLibraryHandlers(): void {
  library.initLibrary()

  // 变更广播（等价 Android 的 playlistsFlow / favoritesFlow）
  if (!wired) {
    library.onLibraryChange(() => sendToRenderer(IpcChannels.LIBRARY_CHANGED))
    wired = true
  }

  handle(IpcChannels.LIBRARY_PLAYLISTS, (): LocalPlaylist[] => library.getPlaylists())
  handle(
    IpcChannels.LIBRARY_PLAYLIST_SONGS,
    (playlistId: number, limit?: number, offset?: number): MusicItem[] =>
      library.queryPlaylistSongs(playlistId, limit, offset)
  )
  handle(
    IpcChannels.LIBRARY_CREATE_PLAYLIST,
    (name: string, opts?: { remoteSource?: string; remoteId?: string; autoRefresh?: boolean }) =>
      library.createPlaylist(name, opts)
  )
  handle(IpcChannels.LIBRARY_DELETE_PLAYLIST, (playlistId: number) =>
    library.deletePlaylist(playlistId)
  )
  handle(IpcChannels.LIBRARY_RENAME_PLAYLIST, (playlistId: number, newName: string) =>
    library.renamePlaylist(playlistId, newName)
  )
  handle(IpcChannels.LIBRARY_ADD_TO_PLAYLIST, (playlistId: number, item: MusicItem) =>
    library.addToPlaylist(playlistId, item)
  )
  handle(IpcChannels.LIBRARY_REMOVE_FROM_PLAYLIST, (playlistId: number, item: MusicItem) =>
    library.removeFromPlaylist(playlistId, item)
  )
  handle(
    IpcChannels.LIBRARY_MOVE_SONG,
    (playlistId: number, item: MusicItem, newPosition: number) =>
      library.moveSongInPlaylist(playlistId, item, newPosition)
  )
  handle(IpcChannels.LIBRARY_MOVE_PLAYLIST, (playlistId: number, targetIndex: number) =>
    library.movePlaylist(playlistId, targetIndex)
  )
  handle(IpcChannels.LIBRARY_IS_FAVORITE, (item: MusicItem): boolean => library.isFavorite(item))
  handle(IpcChannels.LIBRARY_TOGGLE_FAVORITE, (item: MusicItem): boolean =>
    library.toggleFavorite(item)
  )
  handle(IpcChannels.LIBRARY_ADD_TO_TRIAL, (item: MusicItem, atHead: boolean) =>
    library.addToTrial(item, atHead)
  )
  handle(IpcChannels.LIBRARY_TRIAL_SONGS, (): MusicItem[] => library.queryTrialSongs())
  // 歌词/封面重定向（对应安卓 LocalMusicStore 的 getRedirect/setRedirect/clearRedirect）
  handle(
    IpcChannels.LIBRARY_GET_REDIRECT,
    (item: MusicItem): MusicItem | null => library.getRedirect(item) ?? null
  )
  handle(IpcChannels.LIBRARY_SET_REDIRECT, (item: MusicItem, target: MusicItem) =>
    library.setRedirect(item, target)
  )
  handle(IpcChannels.LIBRARY_CLEAR_REDIRECT, (item: MusicItem) => library.clearRedirect(item))
  handle(
    IpcChannels.LIBRARY_SORT_SONGS,
    (playlistId: number, field: PlaylistSortField, order: PlaylistSortOrder) =>
      library.sortPlaylistSongs(playlistId, field, order)
  )
  handle(IpcChannels.LIBRARY_REPLACE_SONGS, (playlistId: number, items: MusicItem[]) =>
    library.replacePlaylistSongs(playlistId, items)
  )
  // 添加本地歌曲：弹文件框 → 解析标签 → 逐首入歌单；返回统计（null=用户取消）
  handle(
    IpcChannels.LIBRARY_ADD_LOCAL_SONGS,
    async (playlistId: number): Promise<{ added: number; skipped: number } | null> => {
      const paths = await pickLocalSongs()
      if (!paths) return null
      let added = 0
      let skipped = 0
      const before = new Set(library.queryPlaylistSongs(playlistId).map((m) => `${m.id}_${m.type}`))
      for (const p of paths) {
        try {
          const item = await parseLocalSong(p)
          if (before.has(`${item.id}_${item.type}`)) {
            skipped++
            continue
          }
          library.addToPlaylist(playlistId, item)
          added++
        } catch {
          skipped++
        }
      }
      return { added, skipped }
    }
  )
  handle(
    IpcChannels.LIBRARY_SCAN_LOCAL_DIRECTORY,
    async (playlistId: number): Promise<{ taskId: string } | null> => {
      const running = activeTaskId()
      if (running) return { taskId: running }
      const directory = await pickLocalDirectory()
      if (!directory) return null

      const taskId = randomUUID()
      const controller = new AbortController()
      activeScans.set(taskId, { playlistId, controller })
      void runScan(taskId, playlistId, directory, controller)
      return { taskId }
    }
  )
  handle(
    IpcChannels.LIBRARY_ENRICH_LOCAL,
    async (playlistId: number): Promise<{ taskId: string } | null> => {
      const running = activeTaskId()
      if (running) return { taskId: running }
      const items = library
        .queryPlaylistSongs(playlistId)
        .filter((item): item is LocalMusicItem => item.type === 'local')
      if (!items.length) return null
      const taskId = randomUUID()
      const controller = new AbortController()
      activeEnrichments.set(taskId, { playlistId, controller })
      void runEnrichment(taskId, items, controller)
      return { taskId }
    }
  )
  handle(IpcChannels.LIBRARY_CANCEL_SCAN, (taskId: string): boolean => {
    const task = activeScans.get(taskId)
    if (task) {
      task.controller.abort()
      return true
    }
    const enrichment = activeEnrichments.get(taskId)
    if (!enrichment) return false
    enrichment.controller.abort()
    return true
  })
}

async function runScan(
  taskId: string,
  playlistId: number,
  directory: string,
  controller: AbortController
): Promise<void> {
  let lastProgressAt = 0
  const forward = (progress: LocalScanProgress): void => {
    const now = Date.now()
    if (progress.done < progress.total && now - lastProgressAt < 80) return
    lastProgressAt = now
    emitScanProgress({ taskId, taskKind: 'scan', ...progress, added: 0 })
  }
  emitScanProgress({
    taskId,
    taskKind: 'scan',
    phase: 'collecting',
    done: 0,
    total: 0,
    added: 0,
    skipped: 0
  })
  try {
    const scan = await scanLocalSongsProgressive(directory, controller.signal, forward)
    if (scan.cancelled || controller.signal.aborted) {
      emitScanProgress({
        taskId,
        taskKind: 'scan',
        phase: 'cancelled',
        done: 0,
        total: 0,
        added: 0,
        skipped: scan.skipped
      })
      return
    }
    emitScanProgress({
      taskId,
      taskKind: 'scan',
      phase: 'committing',
      done: scan.items.length,
      total: scan.items.length,
      added: 0,
      skipped: scan.skipped
    })
    if (controller.signal.aborted) {
      emitScanProgress({
        taskId,
        taskKind: 'scan',
        phase: 'cancelled',
        done: 0,
        total: 0,
        added: 0,
        skipped: scan.skipped
      })
      return
    }
    const result = library.addToPlaylistBatch(playlistId, scan.items)
    emitScanProgress({
      taskId,
      taskKind: 'scan',
      phase: 'done',
      done: scan.items.length,
      total: scan.items.length,
      added: result.added,
      skipped: scan.skipped + result.skipped
    })
  } catch (error) {
    emitScanProgress({
      taskId,
      taskKind: 'scan',
      phase: 'error',
      done: 0,
      total: 0,
      added: 0,
      skipped: 0,
      error: error instanceof Error ? error.message : String(error)
    })
  } finally {
    activeScans.delete(taskId)
  }
}

async function runEnrichment(
  taskId: string,
  items: LocalMusicItem[],
  controller: AbortController
): Promise<void> {
  let lastProgressAt = 0
  const pending: LocalMusicItem[] = []
  const flush = (): void => {
    if (!pending.length) return
    library.updateSongInfos(pending.splice(0))
  }
  emitScanProgress({
    taskId,
    taskKind: 'enrich',
    phase: 'enriching',
    done: 0,
    total: items.length,
    added: 0,
    skipped: 0
  })
  try {
    const result = await enrichLocalSongs(
      items,
      controller.signal,
      (progress) => {
        const now = Date.now()
        if (progress.done < progress.total && now - lastProgressAt < 80) return
        lastProgressAt = now
        emitScanProgress({
          taskId,
          taskKind: 'enrich',
          phase: 'enriching',
          done: progress.done,
          total: progress.total,
          added: progress.updated,
          skipped: progress.skipped,
          currentPath: progress.currentPath
        })
      },
      (item) => {
        pending.push(item)
        if (pending.length >= 8) flush()
      }
    )
    flush()
    emitScanProgress({
      taskId,
      taskKind: 'enrich',
      phase: result.cancelled ? 'cancelled' : 'done',
      done: result.cancelled ? 0 : items.length,
      total: items.length,
      added: result.updated,
      skipped: result.skipped
    })
  } catch (error) {
    flush()
    emitScanProgress({
      taskId,
      taskKind: 'enrich',
      phase: 'error',
      done: 0,
      total: items.length,
      added: 0,
      skipped: 0,
      error: error instanceof Error ? error.message : String(error)
    })
  } finally {
    activeEnrichments.delete(taskId)
  }
}
