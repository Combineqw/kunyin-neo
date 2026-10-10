import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'

console.log('Running M53 Tauri 2 Architecture & Full-Featured UI Migration Tests...')

const root = process.cwd()
const tauriConfPath = path.join(root, 'src-tauri', 'tauri.conf.json')
const tauriCargoPath = path.join(root, 'src-tauri', 'Cargo.toml')
const tauriMainPath = path.join(root, 'src-tauri', 'src', 'main.rs')
const tauriBridgePath = path.join(root, 'src/renderer/src', 'tauri-bridge.ts')
const rendererMainPath = path.join(root, 'src/renderer/src', 'main.ts')
const rendererDistIndex = path.join(root, 'out', 'renderer', 'index.html')

// 1. Verify tauri.conf.json configuration
console.log('1. 验证 src-tauri/tauri.conf.json 配置...')
assert.ok(fs.existsSync(tauriConfPath), 'tauri.conf.json 必须存在')
const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'))
assert.equal(tauriConf.build.frontendDist, '../out/renderer', 'frontendDist 必须指向全功能 Vue 构建产物 ../out/renderer')
assert.equal(tauriConf.productName, '昆音 (Kunyin Neo)', '产品名称必须对齐')
assert.equal(tauriConf.version, '1.1.0', '版本号必须为 1.1.0')
assert.equal(tauriConf.app.windows[0].title, '昆音 (Kunyin Neo)', '窗口标题必须对齐')
assert.ok(tauriConf.app.withGlobalTauri, '必须启用 withGlobalTauri 支持前端全局通信')

// 2. Verify src-tauri/Cargo.toml dependencies
console.log('2. 验证 src-tauri/Cargo.toml 原生 Rust 依赖...')
assert.ok(fs.existsSync(tauriCargoPath), 'src-tauri/Cargo.toml 必须存在')
const cargoContent = fs.readFileSync(tauriCargoPath, 'utf8')
assert.ok(cargoContent.includes('aurora-core = { path = "../crates/aurora-core" }'), '必须依赖 aurora-core')
assert.ok(cargoContent.includes('aurora-library = { path = "../crates/aurora-library" }'), '必须依赖 aurora-library')
assert.ok(cargoContent.includes('aurora-player = { path = "../crates/aurora-player" }'), '必须依赖 aurora-player')
assert.ok(cargoContent.includes('aurora-audio = { path = "../crates/aurora-audio" }'), '必须依赖 aurora-audio')

// 3. Verify src-tauri/src/main.rs commands
console.log('3. 验证 src-tauri/src/main.rs 指令注册...')
assert.ok(fs.existsSync(tauriMainPath), 'src-tauri/src/main.rs 必须存在')
const mainContent = fs.readFileSync(tauriMainPath, 'utf8')
const expectedCommands = [
  'app_version',
  'app_platform',
  'lyric_parse_syllables',
  'lyric_compute_syllable_progress',
  'lyric_render_desktop_frame',
  'compute_mini_player_appearance',
  'compute_desktop_lyric_appearance',
  'network_stream_headers',
  'network_plan_stream_range',
  'window_minimize',
  'window_close',
  'window_toggle_maximize',
  'player_play',
  'player_pause',
  'player_seek',
  'player_snapshot',
  'library_list_playlists',
  'library_query_songs'
]

for (const cmd of expectedCommands) {
  assert.ok(mainContent.includes(`fn ${cmd}`), `main.rs 必须声明命令函数: ${cmd}`)
  assert.ok(mainContent.includes(cmd), `main.rs invoke_handler 必须注册命令: ${cmd}`)
}

// 4. Verify src/renderer/src/tauri-bridge.ts
console.log('4. 验证前端 Tauri 桥接适配器 tauri-bridge.ts...')
assert.ok(fs.existsSync(tauriBridgePath), 'src/renderer/src/tauri-bridge.ts 必须存在')
const bridgeContent = fs.readFileSync(tauriBridgePath, 'utf8')
assert.ok(bridgeContent.includes('export function isTauriEnvironment'), '必须导出 isTauriEnvironment')
assert.ok(bridgeContent.includes('export function initTauriBridge'), '必须导出 initTauriBridge')
assert.ok(bridgeContent.includes('tauriInvoke'), '必须声明 tauriInvoke 通道')
assert.ok(bridgeContent.includes('window_minimize'), '必须映射窗口最小化指令')
assert.ok(bridgeContent.includes('library_list_playlists'), '必须映射曲库歌单指令')

// 5. Verify src/renderer/src/main.ts integration
console.log('5. 验证 main.ts 启动时注入...')
assert.ok(fs.existsSync(rendererMainPath), 'src/renderer/src/main.ts 必须存在')
const rendererMainContent = fs.readFileSync(rendererMainPath, 'utf8')
assert.ok(rendererMainContent.includes("import { initTauriBridge } from './tauri-bridge'"), '必须引入 initTauriBridge')
assert.ok(rendererMainContent.includes('initTauriBridge()'), '必须执行 initTauriBridge()')

// 6. Verify built assets
console.log('6. 验证 out/renderer 生产构建产物...')
assert.ok(fs.existsSync(rendererDistIndex), 'out/renderer/index.html 生产构建产物必须存在')

console.log('All M53 Tauri 2 Architecture & Full-Featured UI Migration Tests passed successfully!')
