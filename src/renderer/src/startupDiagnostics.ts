/**
 * 渲染进程启动前诊断。
 * 保持为打包后的外部模块，使现有 CSP 的 script-src 'self' 可以执行；
 * Vue 尚未挂载时若发生同步错误或未处理拒绝，向启动占位区写入可见错误而非白屏。
 */
function showStartupFailure(reason: unknown): void {
  const root = document.getElementById('app')
  if (!root || root.dataset.mounted === 'true') return
  const title = document.createElement('h2')
  title.textContent = '坤音neo 启动失败'
  const detail = document.createElement('pre')
  detail.textContent = String(reason || '未知渲染错误')
  const fallback = document.createElement('div')
  fallback.id = 'startup-fallback'
  fallback.append(title, detail)
  root.replaceChildren(fallback)
}

// 修复: 原内联诊断被 CSP script-src 'self' 阻断；外部模块能在不放宽 CSP 的前提下捕获首屏异常。
window.addEventListener('error', (event) => {
  showStartupFailure(event.error ?? event.message)
})
window.addEventListener('unhandledrejection', (event) => {
  showStartupFailure(event.reason)
})
