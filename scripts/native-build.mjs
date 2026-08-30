/**
 * 编译 aurora-native 的包装脚本。
 *
 * 存在的唯一理由：rustup 安装时用了 --no-modify-path，cargo 不在系统 PATH 里，
 * 直接 `napi build` 会报 'cargo' is not recognized。这里在启动子进程前把
 * %USERPROFILE%\.cargo\bin 注入 PATH，用户就不需要「记得先设环境变量」。
 *
 * 若将来 cargo 已进系统 PATH，这段注入是无害的幂等操作。
 */

import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, delimiter } from 'node:path'
import { homedir } from 'node:os'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const crateDir = resolve(here, '..', 'crates', 'aurora-native')
const isWinPlatform = process.platform === 'win32'

/** 找到 cargo 所在目录：优先系统 PATH，其次 rustup 默认位置 */
function resolveCargoBin() {
  const probe = spawnSync(process.platform === 'win32' ? 'where' : 'which', ['cargo'], {
    encoding: 'utf8',
  })
  if (probe.status === 0) return null // 已在 PATH 中，无需注入

  const candidates = [
    join(homedir(), '.cargo', 'bin'),
    process.env.CARGO_HOME ? join(process.env.CARGO_HOME, 'bin') : null,
  ].filter(Boolean)

  for (const dir of candidates) {
    const exe = join(dir, process.platform === 'win32' ? 'cargo.exe' : 'cargo')
    if (existsSync(exe)) return dir
  }
  return undefined // 明确表示「装都没装」，与 null（已在 PATH）区分
}

const cargoBin = resolveCargoBin()
if (cargoBin === undefined) {
  console.error('')
  console.error('  ✗ 找不到 cargo。请先安装 Rust 工具链：https://rustup.rs')
  console.error('    装好后重新运行本命令即可，无需手动配置环境变量。')
  console.error('')
  process.exit(1)
}

const env = { ...process.env }
const injected = []

if (cargoBin) {
  env.PATH = cargoBin + delimiter + (env.PATH ?? '')
  injected.push('cargo → ' + cargoBin)
}

// napi.cmd 内部会直接调用 "node"。本机的 node 可能不在系统 PATH 里
// （版本管理器 / 托管运行时启动时就是这样），那样 .cmd 会报
// '"node"' is not recognized。
// 这里无条件把当前进程的 node 目录前置：即使 PATH 里已有 node，
// 前置的也是「正在运行本脚本的那个 node」，版本更确定，且操作幂等。
const nodeDir = dirname(process.execPath)
env.PATH = nodeDir + delimiter + (env.PATH ?? '')
injected.push('node → ' + nodeDir)

if (injected.length) {
  console.log('  已临时注入 PATH：' + injected.join('，'))
}

// 走本地 napi cli；--release 产出优化后的 .node。
// Windows 下 .cmd 必须经 cmd.exe 启动，但不能同时用 shell:true —— 那会让
// 带空格/引号的路径被二次解析，报 '"node"' is not recognized。
// 这里显式走 cmd /c 并把参数交给 spawnSync 逐个转义，最稳。
const napiBin = join(crateDir, 'node_modules', '.bin', isWinPlatform ? 'napi.cmd' : 'napi')
const buildArgs = ['build', '--platform', '--release']

let runner
let args
if (existsSync(napiBin)) {
  runner = isWinPlatform ? process.env.ComSpec || 'cmd.exe' : napiBin
  args = isWinPlatform ? ['/c', napiBin, ...buildArgs] : buildArgs
} else {
  runner = isWinPlatform ? process.env.ComSpec || 'cmd.exe' : 'npx'
  args = isWinPlatform ? ['/c', 'npx', 'napi', ...buildArgs] : ['napi', ...buildArgs]
}

const r = spawnSync(runner, args, {
  cwd: crateDir,
  env,
  stdio: 'inherit',
  shell: false,
})
if (r.error) {
  console.error('  ✗ 启动编译器失败：' + r.error.message)
  process.exit(1)
}
process.exit(r.status ?? 1)