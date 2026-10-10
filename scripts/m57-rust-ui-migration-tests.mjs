import assert from 'node:assert/strict'
import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'

console.log('Running M57 Pure Rust UI Migration & Provider URL Resolution Verification Tests...')

// 1. 验证 aurora-core 中 provider.rs 的下沉实现
console.log('1. 验证 aurora-core 中 URL 提取、音质降级选择与媒体解析契约...')
const coreProviderPath = join(process.cwd(), 'crates', 'aurora-core', 'src', 'provider.rs')
assert.ok(existsSync(coreProviderPath), 'crates/aurora-core/src/provider.rs 必须存在')
const coreProviderContent = readFileSync(coreProviderPath, 'utf8')

assert.ok(coreProviderContent.includes('pub fn extract_backend_music_id('), 'provider.rs 必须包含 extract_backend_music_id')
assert.ok(coreProviderContent.includes('pub fn select_optimal_quality'), 'provider.rs 必须包含 select_optimal_quality')
assert.ok(coreProviderContent.includes('pub fn parse_media_info_response('), 'provider.rs 必须包含 parse_media_info_response')

// 2. 验证 aurora-app 中的推荐、歌单详情、EQ 与迷你播放器组件
console.log('2. 验证 aurora-app 推荐视图 RecommendViewModel 与心动模式...')
const recommendViewPath = join(process.cwd(), 'crates', 'aurora-app', 'src', 'components', 'recommend_view.rs')
assert.ok(existsSync(recommendViewPath), 'recommend_view.rs 必须存在')
const recommendContent = readFileSync(recommendViewPath, 'utf8')
assert.ok(recommendContent.includes('pub struct RecommendViewModel'), '必须包含 RecommendViewModel')
assert.ok(recommendContent.includes('pub struct CuratedPlaylistCard'), '必须包含 CuratedPlaylistCard')
assert.ok(recommendContent.includes('pub fn formatted_play_count(&self)'), '必须包含播放量中文单位格式化逻辑')
assert.ok(recommendContent.includes('pub fn toggle_heartbeat(&mut self)'), '必须包含心动模式切换')

console.log('3. 验证 aurora-app 歌单详情 PlaylistDetailViewModel 与全部播放...')
const playlistViewPath = join(process.cwd(), 'crates', 'aurora-app', 'src', 'components', 'playlist_detail_view.rs')
assert.ok(existsSync(playlistViewPath), 'playlist_detail_view.rs 必须存在')
const playlistContent = readFileSync(playlistViewPath, 'utf8')
assert.ok(playlistContent.includes('pub struct PlaylistDetailViewModel'), '必须包含 PlaylistDetailViewModel')
assert.ok(playlistContent.includes('pub fn formatted_total_duration(&self)'), '必须包含歌单总时长格式化')
assert.ok(playlistContent.includes('pub fn filter_tracks(&self'), '必须包含歌单内搜索过滤')

console.log('4. 验证 aurora-app 10 段图示均衡器 EqVisualizerModel 与频响曲线坐标生成...')
const eqModalPath = join(process.cwd(), 'crates', 'aurora-app', 'src', 'components', 'eq_modal.rs')
assert.ok(existsSync(eqModalPath), 'eq_modal.rs 必须存在')
const eqContent = readFileSync(eqModalPath, 'utf8')
assert.ok(eqContent.includes('pub struct EqVisualizerModel'), '必须包含 EqVisualizerModel')
assert.ok(eqContent.includes('pub fn compute_curve_points('), '必须包含曲线坐标生成算法')
assert.ok(eqContent.includes('pub const MIN_GAIN_DB: f32 = -12.0;'), '必须约束增益下限 -12dB')
assert.ok(eqContent.includes('pub const MAX_GAIN_DB: f32 = 12.0;'), '必须约束增益上限 +12dB')

console.log('5. 验证 aurora-app 迷你播放器 MiniPlayerViewModel 与旋转状态机...')
const miniPlayerPath = join(process.cwd(), 'crates', 'aurora-app', 'src', 'components', 'mini_player.rs')
assert.ok(existsSync(miniPlayerPath), 'mini_player.rs 必须存在')
const miniContent = readFileSync(miniPlayerPath, 'utf8')
assert.ok(miniContent.includes('pub struct MiniPlayerViewModel'), '必须包含 MiniPlayerViewModel')
assert.ok(miniContent.includes('pub fn tick_rotation(&mut self'), '必须包含黑胶/封面旋转推进逻辑')
assert.ok(miniContent.includes('pub fn set_opacity(&mut self'), '必须包含透明度约束')

console.log('6. 验证 aurora-app window 路由整合与触控点热区...')
const windowPath = join(process.cwd(), 'crates', 'aurora-app', 'src', 'window.rs')
const windowContent = readFileSync(windowPath, 'utf8')
assert.ok(windowContent.includes('AppRoute::Discover =>'), 'window.rs 必须接线 Discover 视图渲染')
assert.ok(windowContent.includes('AppRoute::Playlist(_) =>'), 'window.rs 必须接线 Playlist 详情视图渲染')
assert.ok(windowContent.includes('AppRoute::Settings =>'), 'window.rs 必须接线 10段图示均衡器渲染')
assert.ok(windowContent.includes('AppRoute::Search =>'), 'window.rs 必须接线多源搜索渲染')
assert.ok(windowContent.includes('WindowDisplayMode::MiniPlayer'), 'window.rs 必须接线迷你小窗模式')

console.log('7. 验证 TypeScript 原生桥接器与 getUrl.ts 接线...')
const bridgeRuntime = await import('../src/main/native/bridge-runtime.js')
assert.strictEqual(typeof bridgeRuntime.nativeExtractBackendMusicId, 'function')
assert.strictEqual(typeof bridgeRuntime.nativeSelectOptimalQuality, 'function')
assert.strictEqual(typeof bridgeRuntime.nativeParseMediaInfoResponse, 'function')

// 验证提取规则
assert.strictEqual(bridgeRuntime.nativeExtractBackendMusicId('qq', '12', '003mid', null), '003mid')
assert.strictEqual(bridgeRuntime.nativeExtractBackendMusicId('kg', '34', null, 'AABBCC1122'), 'aabbcc1122')
assert.strictEqual(bridgeRuntime.nativeExtractBackendMusicId('wy', '5678', null, null), '5678')

// 验证降级选择规则
assert.strictEqual(bridgeRuntime.nativeSelectOptimalQuality(['128k', '320k', 'flac'], 'flac'), 'flac')
assert.strictEqual(bridgeRuntime.nativeSelectOptimalQuality(['128k', '320k', 'flac'], 'hires'), 'flac')

// 验证解析规则
const mockResp = { code: 200, url: 'https://cdn.example.com/audio.flac', _cacheTTL: 300 }
const parsed = bridgeRuntime.nativeParseMediaInfoResponse('qq', 'flac', mockResp, 1000)
assert.ok(parsed && parsed.isSuccess === true)
assert.strictEqual(parsed.playUrl, 'https://cdn.example.com/audio.flac')

console.log('All M57 Pure Rust UI Migration & Provider URL Resolution Verification Tests passed successfully!')
