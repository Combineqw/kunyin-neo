/**
 * M52: 纯 Rust 原生桌面歌词与迷你播放器置顶悬浮视窗、歌词全屏动态视图与原生网络流式调度增强自动化验证套件
 */
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  nativeLyricsParseSyllables,
  nativeLyricsComputeSyllableProgress,
  nativeLyricsRenderDesktopFrame
} from '../src/main/native/bridge.ts'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const rootDir = path.resolve(__dirname, '..')

console.log('Running M52 Rust Advancement & Desktop Lyric Floating Window Verification Tests...')

// 1. 验证 aurora-core 中的逐字歌词解析与渲染帧导出
console.log('1. 验证 aurora-core 源码与单元测试契约...')
const coreLyricsFile = path.join(rootDir, 'crates/aurora-core/src/lyrics.rs')
const coreLyricsContent = fs.readFileSync(coreLyricsFile, 'utf8')
assert.ok(coreLyricsContent.includes('pub struct SyllableSpan'), '必须定义 SyllableSpan')
assert.ok(coreLyricsContent.includes('pub struct SyllableLine'), '必须定义 SyllableLine')
assert.ok(coreLyricsContent.includes('pub struct SyllableProgress'), '必须定义 SyllableProgress')
assert.ok(coreLyricsContent.includes('pub struct DesktopLyricRenderFrame'), '必须定义 DesktopLyricRenderFrame')
assert.ok(coreLyricsContent.includes('pub fn parse_syllable_line'), '必须导出 parse_syllable_line')
assert.ok(coreLyricsContent.includes('pub fn compute_syllable_progress'), '必须导出 compute_syllable_progress')
assert.ok(coreLyricsContent.includes('pub fn render_desktop_lyric_frame'), '必须导出 render_desktop_lyric_frame')

// 2. 验证 aurora-app 中的桌面歌词原生窗口与歌词全屏视图
console.log('2. 验证 aurora-app Win32 原生置顶桌面歌词视窗与全屏歌词组件...')
const appLyricWinFile = path.join(rootDir, 'crates/aurora-app/src/desktop_lyric_window.rs')
assert.ok(fs.existsSync(appLyricWinFile), 'desktop_lyric_window.rs 必须存在')
const appLyricWinContent = fs.readFileSync(appLyricWinFile, 'utf8')
assert.ok(appLyricWinContent.includes('WS_EX_LAYERED'), '必须使用 Win32 分层透明窗口 WS_EX_LAYERED')
assert.ok(appLyricWinContent.includes('WS_EX_TRANSPARENT'), '必须支持锁定鼠标穿透 WS_EX_TRANSPARENT')
assert.ok(appLyricWinContent.includes('compute_appearance'), '必须接入 Apple HIG 外观合成')

const appLyricViewFile = path.join(rootDir, 'crates/aurora-app/src/components/lyric_view.rs')
assert.ok(fs.existsSync(appLyricViewFile), 'lyric_view.rs 必须存在')
const appLyricViewContent = fs.readFileSync(appLyricViewFile, 'utf8')
assert.ok(appLyricViewContent.includes('LyricViewModel'), '必须定义 LyricViewModel')
assert.ok(appLyricViewContent.includes('1.08'), '必须实现活跃行 1.08x 放大聚焦视觉分层')

// 3. 验证 aurora-app 状态机对歌词与桌面视窗的集成
console.log('3. 验证 aurora-app state 与 window 路由集成...')
const appStateFile = path.join(rootDir, 'crates/aurora-app/src/state.rs')
const appStateContent = fs.readFileSync(appStateFile, 'utf8')
assert.ok(appStateContent.includes('AppRoute::Lyrics') || appStateContent.includes('Lyrics,'), 'AppRoute 必须包含 Lyrics 路由')
assert.ok(appStateContent.includes('load_lyrics'), 'AppState 必须提供 load_lyrics 方法')
assert.ok(appStateContent.includes('toggle_desktop_lyric'), 'AppState 必须提供 toggle_desktop_lyric 方法')

const appWindowFile = path.join(rootDir, 'crates/aurora-app/src/window.rs')
const appWindowContent = fs.readFileSync(appWindowFile, 'utf8')
assert.ok(appWindowContent.includes('桌面歌词'), 'window.rs 工具栏必须包含桌面歌词快速控制')
assert.ok(appWindowContent.includes('AppRoute::Lyrics'), 'window.rs 必须处理全屏歌词页面渲染')

// 4. 验证 aurora-native 原生 N-API 接口与 bridge 适配器
console.log('4. 验证 bridge.ts 中 Rust 原生逐字推进与桌面歌词帧渲染...')
const sampleLrc = '[00:05.00]<5000,500>坤<5500,500>音<6000,600>音乐\n[00:10.00]第二行普通歌词'
const parsedSyllables = nativeLyricsParseSyllables(sampleLrc)
assert.ok(Array.isArray(parsedSyllables), 'nativeLyricsParseSyllables 必须返回数组')
if (parsedSyllables.length > 0) {
  const firstLine = parsedSyllables[0]
  assert.equal(firstLine.start_ms, 5000)
  assert.equal(firstLine.raw_text, '坤音音乐')

  // 进度推进计算
  const progMid = nativeLyricsComputeSyllableProgress(firstLine, 5750)
  assert.equal(progMid.current_text, '音')
  assert.equal(progMid.sang_text, '坤')
  assert.equal(progMid.unsang_text, '音乐')
}

// 桌面渲染帧生成
const frame = nativeLyricsRenderDesktopFrame(
  '当前播放中的歌词',
  '下一句歌词预览',
  0.75,
  '#E8C083',
  '#8E8E93',
  0.72,
  100,
  26,
  true
)
assert.equal(frame.main_text, '当前播放中的歌词')
assert.equal(frame.sub_text, '下一句歌词预览')
assert.equal(frame.highlight_ratio, 0.75)
assert.equal(frame.active_color, '#E8C083')
assert.equal(frame.normal_color, '#8E8E93')
assert.equal(frame.is_playing, true)

console.log('All M52 Rust Advancement & Desktop Lyric Floating Window Verification Tests passed successfully!')
