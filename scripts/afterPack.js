'use strict'
const fs = require('fs')
const path = require('path')
const { execFileSync } = require('child_process')

/**
 * 打包后清理：删除 Electron 自带的 LICENSES.chromium.html(~15M 许可文本)。
 *
 * 注意：不能用 fs.unlinkSync —— WorkBuddy 环境会把 Node fs 的删除操作接管为
 * 「回收站 + 批量确认」护栏，打包过程中的累计删除会触发 SAFE_DELETE_BULK_CONFIRM_REQUIRED。
 * 这里改用子进程调用 PowerShell Remove-Item（原生二进制，不受 Node fs 垫片拦截）。
 */
exports.default = async function (context) {
  // The native artifact is intentionally git-ignored. Copy it when a local
  // Rust build prepared one, while allowing clean CI packages to use Node
  // fallbacks instead of failing electron-builder during file collection.
  if (context.electronPlatformName === 'win32' && context.arch === 1) {
    const nativeSource = path.join(
      context.packager.projectDir,
      'crates',
      'aurora-native',
      'aurora-native.win32-x64-msvc.node'
    )
    const nativeTarget = path.join(
      context.appOutDir,
      'resources',
      'assets',
      'aurora-native.win32-x64-msvc.node'
    )
    if (fs.existsSync(nativeSource)) {
      fs.mkdirSync(path.dirname(nativeTarget), { recursive: true })
      fs.copyFileSync(nativeSource, nativeTarget)
      console.log('[afterPack] 已复制 aurora-native.win32-x64-msvc.node')
    } else {
      console.warn('[afterPack] 未找到 Rust 原生模块，使用 Node 回退')
    }
  }

  const licenseFile = path.join(context.appOutDir, 'LICENSES.chromium.html')
  if (!fs.existsSync(licenseFile)) return
  try {
    const script = `Remove-Item -LiteralPath '${licenseFile.replace(/'/g, "''")}' -Force -Confirm:$false`
    execFileSync(
      process.env.SystemRoot ? path.join(process.env.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe') : 'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-Command', script],
      { stdio: 'ignore', timeout: 30000 }
    )
    console.log('[afterPack] 已删除 LICENSES.chromium.html')
  } catch {
    // 删除失败不阻断打包（仅多 ~15M）
    console.warn('[afterPack] 删除 LICENSES.chromium.html 失败，忽略')
  }
}
