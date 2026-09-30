const button = document.getElementById('run')
const status = document.getElementById('status')
const output = document.getElementById('output')
const scanButton = document.getElementById('scan')
const scanStatus = document.getElementById('scan-status')
const scanOutput = document.getElementById('scan-output')
const settingsOutput = document.getElementById('settings-output')
const capabilities = document.getElementById('capabilities')

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

invoke('native_capabilities').then((value) => {
  capabilities.textContent = JSON.stringify(value, null, 2)
}).catch((error) => {
  capabilities.textContent = String(error)
})
