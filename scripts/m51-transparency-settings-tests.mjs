/**
 * M51 验证测试套件：
 * 1. 迷你播放器背景透明度与桌面歌词透明度调节（设置契约、响应式与 Apple Design 滑块）
 * 2. 桌面歌词与迷你播放器组件中的动态背景透明度与毛玻璃渲染接线
 * 3. 下沉到 Rust 的桌面歌词时间轴高精度二分查找、外观参数合成与在线网络跨源请求头调度
 */
import assert from 'node:assert/strict'
import { readFileSync, existsSync } from 'node:fs'
import { resolve, join } from 'node:path'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const nativePath = resolve('crates/aurora-native/aurora-native.win32-x64-msvc.node')
assert.ok(existsSync(nativePath), `必须存在编译好的原生模块: ${nativePath}`)
const binding = require(nativePath)

console.log('Running M51 Transparency & Native Rust Downsink Verification Tests...')

// 1. 设置模型默认值与范围检查
console.log('1. 验证默认设置与透明度字段契约...')
const settingsContent = readFileSync(resolve('src/common/types/settings.ts'), 'utf-8')
assert.match(
  settingsContent,
  /miniPlayerOpacity:\s*number/,
  'AppSettings.player 必须声明 miniPlayerOpacity: number'
)
assert.match(
  settingsContent,
  /miniPlayerOpacity:\s*72/,
  'DEFAULT_SETTINGS.player 必须默认 miniPlayerOpacity 为 72'
)
assert.match(
  settingsContent,
  /desktopBgOpacity:\s*0\.28/,
  'DEFAULT_SETTINGS.lyrics 必须默认 desktopBgOpacity 为 0.28'
)
assert.match(
  settingsContent,
  /desktopOpacity:\s*100/,
  'DEFAULT_SETTINGS.lyrics 必须默认 desktopOpacity 为 100'
)

// 2. 检查 UI 组件中的 Apple Design 规范与透明度调节控件
console.log('2. 验证 SettingPlay.vue 与 SettingDesktopLyric.vue 中的 Apple Design 控件与即时响应...')
const playSettingContent = readFileSync(
  resolve('src/renderer/src/views/settings/SettingPlay.vue'),
  'utf-8'
)
assert.ok(
  playSettingContent.includes('id="setting_mini_player_opacity"'),
  'SettingPlay.vue 应包含 setting_mini_player_opacity 滑块'
)
assert.ok(
  playSettingContent.includes('@input="setMiniPlayerOpacity"'),
  'SettingPlay.vue 应包含 @input 即时无级响应'
)
assert.ok(
  playSettingContent.includes('mini-player-opacity-wrap') &&
  playSettingContent.includes('apple-badge') &&
  playSettingContent.includes('apple-range-input'),
  'SettingPlay.vue 应遵循 Apple Design 滑块卡片与徽章样式'
)

const dlSettingContent = readFileSync(
  resolve('src/renderer/src/views/settings/SettingDesktopLyric.vue'),
  'utf-8'
)
assert.ok(
  dlSettingContent.includes('@input="setNumber(\'desktopOpacity\''),
  'SettingDesktopLyric.vue 歌词文字不透明度应支持 @input 即时响应'
)
assert.ok(
  dlSettingContent.includes('@input="setNumber(\'desktopBgOpacity\''),
  'SettingDesktopLyric.vue 桌面歌词背景不透明度应支持 @input 即时响应'
)
assert.ok(
  dlSettingContent.includes('max="100"'),
  'SettingDesktopLyric.vue 背景不透明度应放宽至 100% 全范围'
)

// 3. 检查 MiniPlayer.vue 与 DesktopLyrics.vue 动态样式绑定
console.log('3. 验证 MiniPlayer.vue 与 DesktopLyrics.vue 中的透明度样式绑定...')
const miniPlayerContent = readFileSync(
  resolve('src/renderer/src/mini-player/MiniPlayer.vue'),
  'utf-8'
)
assert.ok(
  miniPlayerContent.includes(':style="miniPlayerStyle"'),
  'MiniPlayer.vue 模板根节点应绑定 miniPlayerStyle'
)
assert.ok(
  miniPlayerContent.includes('miniPlayerOpacity'),
  'MiniPlayer.vue 应持有 miniPlayerOpacity 响应式状态'
)
assert.ok(
  miniPlayerContent.includes('window.api.settings.onChange'),
  'MiniPlayer.vue 应监听主进程推送的 settings.onChange'
)
assert.ok(
  miniPlayerContent.includes('--mp-bg-opacity'),
  'MiniPlayer.vue CSS 应使用 --mp-bg-opacity 控制毛玻璃底板透明度'
)

const dlContent = readFileSync(
  resolve('src/renderer/src/desktop-lyrics/DesktopLyrics.vue'),
  'utf-8'
)
assert.ok(
  dlContent.includes('Math.min(1, s?.desktopBgOpacity'),
  'DesktopLyrics.vue 应允许最大 1.0 的背景不透明度'
)

// 4. 验证 Rust 原生外观参数合成契约
console.log('4. 验证 Rust 原生模块 mini_player_compute_appearance 与 desktop_lyric_compute_appearance 合成...')
const mpApp72 = JSON.parse(binding.miniPlayerComputeAppearance(72))
assert.strictEqual(mpApp72.opacity, 0.72)
assert.strictEqual(mpApp72.bg_rgba, 'rgba(31, 33, 31, 0.72)')

const mpApp10 = JSON.parse(binding.miniPlayerComputeAppearance(10))
assert.strictEqual(mpApp10.opacity, 0.10)
assert.strictEqual(mpApp10.bg_rgba, 'rgba(31, 33, 31, 0.10)')

const mpApp100 = JSON.parse(binding.miniPlayerComputeAppearance(100))
assert.strictEqual(mpApp100.opacity, 1.00)
assert.strictEqual(mpApp100.bg_rgba, 'rgba(31, 33, 31, 1.00)')

const dlApp = JSON.parse(binding.desktopLyricComputeAppearance(0.5, 80, 600, 0.72, 2))
assert.strictEqual(dlApp.bg_opacity, 0.5)
assert.strictEqual(dlApp.text_opacity, 0.8)
assert.strictEqual(dlApp.background_rgba, 'rgba(9, 12, 11, 0.50)')
assert.strictEqual(dlApp.text_opacity_str, '0.80')

// 5. 验证 Rust 原生歌词时间轴二分查找契约
console.log('5. 验证 Rust 原生歌词时间轴高精度二分查找...')
const testLines = [
  { start: 1000, end: 3000, text: '第一句歌词' },
  { start: 3000, end: 7000, text: '第二句高潮' },
  { start: 7000, end: 12000, text: '第三句尾声' }
]
const match1 = JSON.parse(binding.lyricsFindActiveLine(JSON.stringify(testLines), 2000))
assert.ok(match1)
assert.strictEqual(match1.index, 0)
assert.strictEqual(match1.text, '第一句歌词')
assert.strictEqual(match1.next_text, '第二句高潮')
assert.ok(Math.abs(match1.progress - 0.5) < 0.05, '进度应为 50%')

const match2 = JSON.parse(binding.lyricsFindActiveLine(JSON.stringify(testLines), 5000))
assert.ok(match2)
assert.strictEqual(match2.index, 1)
assert.strictEqual(match2.text, '第二句高潮')
assert.strictEqual(match2.prev_text, '第一句歌词')
assert.strictEqual(match2.next_text, '第三句尾声')
assert.ok(Math.abs(match2.progress - 0.5) < 0.05)

// 6. 验证 Rust 在线网络跨源请求头与代理调度契约
console.log('6. 验证 Rust 在线网络跨源请求头与代理调度契约...')
const qqHeaders = JSON.parse(binding.networkBuildHeaders('qq', 'https://y.qq.com/song/123.m4a', 0, 1024))
assert.strictEqual(qqHeaders.provider, 'qq')
assert.strictEqual(qqHeaders.headers['Referer'], 'https://y.qq.com/')
assert.strictEqual(qqHeaders.headers['Range'], 'bytes=0-1024')

const wyHeaders = JSON.parse(binding.networkBuildHeaders('wy', 'https://music.163.com/song.mp3', 1024, null))
assert.strictEqual(wyHeaders.provider, 'wy')
assert.strictEqual(wyHeaders.headers['Referer'], 'https://music.163.com/')
assert.strictEqual(wyHeaders.headers['Range'], 'bytes=1024-')

const rangePlan = JSON.parse(binding.networkPlanStreamRange(0, 500000, 256 * 1024))
assert.strictEqual(rangePlan.start, 0)
assert.strictEqual(rangePlan.end, 262143)
assert.strictEqual(rangePlan.is_final, false)

const validProxy = binding.networkValidateProxy(true, '127.0.0.1', 7890)
assert.strictEqual(validProxy, 'http://127.0.0.1:7890')

const disabledProxy = binding.networkValidateProxy(false, '127.0.0.1', 7890)
assert.strictEqual(disabledProxy, '')

console.log('All M51 Transparency & Native Rust Downsink Verification Tests passed successfully!')
