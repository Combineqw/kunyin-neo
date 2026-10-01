/* eslint-disable @typescript-eslint/explicit-function-return-type */
const button = document.getElementById('run')
const status = document.getElementById('status')
const output = document.getElementById('output')
const scanButton = document.getElementById('scan')
const scanStatus = document.getElementById('scan-status')
const scanOutput = document.getElementById('scan-output')
const settingsOutput = document.getElementById('settings-output')
const capabilities = document.getElementById('capabilities')
const audioCapabilities = document.getElementById('audio-capabilities')
const libraryStatus = document.getElementById('library-status')
const libraryOutput = document.getElementById('library-output')
const playlistSelect = document.getElementById('playlist-select')

const invoke = (command, args) => window.__TAURI__.core.invoke(command, args)

button.addEventListener('click', async () => {
  button.disabled = true
  status.textContent = '正在调用 Rust...'
  try {
    const result = await invoke('parse_lyrics', {
      input: document.getElementById('input').value
    })
    output.textContent = JSON.stringify(result, null, 2)
    status.textContent = `${result.length} 行`
  } catch (error) {
    output.textContent = String(error)
    status.textContent = '调用失败'
  } finally {
    button.disabled = false
  }
})

scanButton.addEventListener('click', async () => {
  const path = document.getElementById('scan-path').value.trim()
  if (!path) {
    scanStatus.textContent = '请输入目录路径'
    return
  }
  scanButton.disabled = true
  scanStatus.textContent = '正在扫描...'
  try {
    const result = await invoke('scan_library', { path })
    scanOutput.textContent = JSON.stringify(result, null, 2)
    scanStatus.textContent = `${result.tracks.length} 首，跳过 ${result.skippedNonAudio} 个非音频文件`
  } catch (error) {
    scanStatus.textContent = '扫描失败'
    scanOutput.textContent = String(error)
  } finally {
    scanButton.disabled = false
  }
})

document.getElementById('read-settings').addEventListener('click', async () => {
  try {
    const result = await invoke('read_settings')
    settingsOutput.textContent = JSON.stringify(result, null, 2)
  } catch (error) {
    settingsOutput.textContent = String(error)
  }
})

document.getElementById('write-settings').addEventListener('click', async () => {
  try {
    const result = await invoke('update_settings', { patch: { nativeMigration: { lastProbe: new Date().toISOString() } } })
    settingsOutput.textContent = JSON.stringify(result, null, 2)
  } catch (error) {
    settingsOutput.textContent = String(error)
  }
})

async function refreshPlaylists() {
  try {
    const playlists = await invoke('library_list_playlists')
    playlistSelect.replaceChildren(...playlists.map((playlist) => {
      const option = document.createElement('option')
      option.value = String(playlist.id)
      option.textContent = `${playlist.name} (${playlist.songCount})`
      return option
    }))
    libraryOutput.textContent = JSON.stringify(playlists, null, 2)
    libraryStatus.textContent = `${playlists.length} 个歌单`
  } catch (error) {
    libraryStatus.textContent = '读取失败'
    libraryOutput.textContent = String(error)
  }
}

document.getElementById('list-playlists').addEventListener('click', refreshPlaylists)
document.getElementById('create-playlist').addEventListener('click', async () => {
  const name = document.getElementById('playlist-name').value.trim()
  if (!name) {
    libraryStatus.textContent = '请输入歌单名称'
    return
  }
  try {
    await invoke('library_create_playlist', { name, autoRefresh: false })
    await refreshPlaylists()
    libraryStatus.textContent = '歌单已创建'
  } catch (error) {
    libraryStatus.textContent = '创建失败'
    libraryOutput.textContent = String(error)
  }
})
document.getElementById('load-songs').addEventListener('click', async () => {
  const playlistId = Number(playlistSelect.value)
  if (!playlistId) return
  try {
    const songs = await invoke('library_query_songs', { playlistId })
    libraryOutput.textContent = JSON.stringify(songs, null, 2)
    libraryStatus.textContent = `${songs.length} 首歌曲`
  } catch (error) {
    libraryStatus.textContent = '读取曲目失败'
    libraryOutput.textContent = String(error)
  }
})

document.getElementById('search-songs').addEventListener('click', async () => {
  const query = document.getElementById('library-search').value.trim()
  if (!query) {
    libraryStatus.textContent = '请输入搜索词'
    return
  }
  try {
    const songs = await invoke('library_search_songs', { query, limit: 50, offset: 0 })
    libraryOutput.textContent = JSON.stringify(songs, null, 2)
    libraryStatus.textContent = `搜索到 ${songs.length} 首歌曲`
  } catch (error) {
    libraryStatus.textContent = '搜索失败'
    libraryOutput.textContent = String(error)
  }
})

document.getElementById('read-metadata').addEventListener('click', async () => {
  const path = document.getElementById('metadata-path').value.trim()
  const status = document.getElementById('metadata-status')
  const output = document.getElementById('metadata-output')
  if (!path) {
    status.textContent = '请输入音频文件路径'
    return
  }
  try {
    const result = await invoke('read_audio_metadata', { path })
    output.textContent = JSON.stringify(result, null, 2)
    status.textContent = result ? 'Rust 标签读取完成' : '此格式不支持或文件没有可读取标签'
  } catch (error) {
    status.textContent = '读取失败'
    output.textContent = String(error)
  }
})

invoke('native_capabilities').then((value) => {
  capabilities.textContent = JSON.stringify(value, null, 2)
}).catch((error) => {
  capabilities.textContent = String(error)
})

invoke('audio_backend_capabilities').then((value) => {
  audioCapabilities.textContent = JSON.stringify(value, null, 2)
}).catch((error) => {
  audioCapabilities.textContent = String(error)
})
