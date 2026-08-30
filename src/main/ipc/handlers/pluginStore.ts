import { dialog } from 'electron'
import { IpcChannels, type PluginActionResult, type PluginStoreEvent } from '@common'
import { handle, sendToAllRenderers } from '../helpers'
import {
  installPlugin,
  listPlugins,
  setPluginEnabled,
  stageLocalPackage,
  toPluginEvent,
  uninstallPlugin
} from '../../plugins/store'
import { addRepo, listRepos, removeRepo, setRepoEnabled } from '../../plugins/repos'
import { getMainWindow } from '../../windows/main'

function broadcast(type: PluginStoreEvent['type'], result: PluginActionResult): void {
  sendToAllRenderers(IpcChannels.PLUGIN_STORE_CHANGED, toPluginEvent(type, result))
}

/** 注册插件本地安装与状态管理。数据源经 PluginSource 抽象，当前仅本地源。 */
export function registerPluginStoreHandlers(): void {
  handle(IpcChannels.PLUGIN_STORE_LIST, () => listPlugins())
  handle(IpcChannels.PLUGIN_STORE_INSTALL, async (id: string) => {
    const result = await installPlugin(id)
    broadcast('installed', result)
    return result
  })
  handle(IpcChannels.PLUGIN_STORE_UNINSTALL, async (id: string) => {
    const result = await uninstallPlugin(id)
    broadcast('uninstalled', result)
    return result
  })
  handle(IpcChannels.PLUGIN_STORE_SET_ENABLED, async (id: string, enabled: boolean) => {
    const result = await setPluginEnabled(id, enabled)
    broadcast(enabled ? 'enabled' : 'disabled', result)
    return result
  })
  // 自定义源管理。增删改后列表要重拉，故复用 PLUGIN_STORE_CHANGED 通知渲染层刷新。
  handle(IpcChannels.PLUGIN_REPO_LIST, () => listRepos())
  handle(IpcChannels.PLUGIN_REPO_ADD, (url: string, name: string) => addRepo(url, name))
  handle(IpcChannels.PLUGIN_REPO_REMOVE, (id: string) => removeRepo(id))
  handle(IpcChannels.PLUGIN_REPO_SET_ENABLED, (id: string, enabled: boolean) =>
    setRepoEnabled(id, enabled)
  )

  handle(IpcChannels.PLUGIN_STORE_STAGE_LOCAL, (filePath: string) => stageLocalPackage(filePath))
  handle(IpcChannels.PLUGIN_STORE_PICK_LOCAL, async (): Promise<PluginActionResult | null> => {
    const result = await dialog.showOpenDialog(getMainWindow() ?? undefined!, {
      title: '选择插件包',
      properties: ['openFile'],
      filters: [{ name: '坤音neo 插件包', extensions: ['zip'] }]
    })
    if (result.canceled || !result.filePaths[0]) return null
    return stageLocalPackage(result.filePaths[0])
  })
}
