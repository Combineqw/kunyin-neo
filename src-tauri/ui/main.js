const button = document.getElementById('run')
const status = document.getElementById('status')
const output = document.getElementById('output')

button.addEventListener('click', async () => {
  button.disabled = true
  status.textContent = '正在调用 Rust...'
  try {
    const result = await window.__TAURI__.core.invoke('parse_lyrics', {
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
