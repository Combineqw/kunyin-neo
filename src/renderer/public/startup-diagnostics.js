/*
 * 主窗口最早期启动诊断。
 * 此文件作为传统外部脚本先于 ES Module 入口执行：若模块图解析、资源加载或 Vue 初始化本身失败，
 * 仍能把错误写入页面，而不会留下无信息白屏。保持独立文件以兼容 CSP 的 script-src 'self'。
 */
;(function () {
  function stringify(reason) {
    if (reason instanceof Error) return reason.stack || reason.message
    if (reason && typeof reason === 'object') {
      try {
        return JSON.stringify(reason, null, 2)
      } catch (_) {
        return String(reason)
      }
    }
    return String(reason || '未知渲染错误')
  }

  function showFailure(reason) {
    var root = document.getElementById('app')
    if (!root || root.dataset.mounted === 'true') return

    var fallback = document.createElement('div')
    fallback.id = 'startup-fallback'
    fallback.style.cssText = 'box-sizing:border-box;min-height:100vh;padding:32px;background:#fff;color:#1d1b20;font:14px/1.6 "Microsoft YaHei",sans-serif;white-space:pre-wrap;word-break:break-word;'

    var title = document.createElement('h2')
    title.textContent = '坤音neo 启动失败'
    title.style.cssText = 'margin:0 0 12px;font-size:20px;'

    var hint = document.createElement('p')
    hint.textContent = '请截取以下错误文本反馈，以定位渲染进程问题：'
    hint.style.cssText = 'margin:0 0 12px;'

    var detail = document.createElement('pre')
    detail.textContent = stringify(reason)
    detail.style.cssText = 'margin:0;padding:16px;border:1px solid #d9d9e3;border-radius:8px;background:#f7f7fb;white-space:pre-wrap;word-break:break-word;'

    fallback.append(title, hint, detail)
    root.replaceChildren(fallback)
  }

  // 修复: 传统外部脚本在模块图解析前执行，能诊断 main.ts 或其依赖本身无法加载的情况。
  window.addEventListener('error', function (event) {
    var target = event.target
    if (target && target !== window && target.tagName) {
      showFailure('资源加载失败: ' + target.tagName + ' ' + (target.src || target.href || '未知资源'))
      return
    }
    showFailure(event.error || event.message)
  }, true)

  window.addEventListener('unhandledrejection', function (event) {
    showFailure(event.reason)
  })

  window.__KUNYIN_STARTUP_DIAGNOSTICS__ = true
})()
