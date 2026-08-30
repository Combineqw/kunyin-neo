'use strict'

// 依赖已由项目工作区准备完成，避免 electron-builder 在当前 Windows 环境中
// 通过 PowerShell 启动 npm 依赖收集器时发生原生子进程崩溃。
exports.default = async function beforeBuild() {
  return false
}
