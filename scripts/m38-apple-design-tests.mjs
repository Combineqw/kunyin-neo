import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const read = (p) => readFileSync(join(root, p), 'utf8')

// 1. Apple Design tokens in base.css
const baseCss = read('src/renderer/src/assets/base.css')
assert.match(baseCss, /--margin-page:\s*16px;/, 'base.css must define --margin-page 16px')
assert.match(baseCss, /--gap-card:\s*12px;/, 'base.css must define --gap-card 12px')
assert.match(baseCss, /--padding-card:\s*16px;/, 'base.css must define --padding-card 16px')
assert.match(baseCss, /--radius-card:\s*16px;/, 'base.css must define --radius-card 16px')
assert.match(
  baseCss,
  /--font-system:\s*-apple-system,\s*BlinkMacSystemFont,\s*'SF Pro Display'/,
  'base.css must define Apple HIG SF Pro typography stack'
)
assert.match(
  baseCss,
  /--glass-thin-saturation:\s*1\.8;/,
  'base.css must boost thin glass saturation to 1.8'
)
assert.match(
  baseCss,
  /--glass-regular-saturation:\s*1\.8;/,
  'base.css must boost regular glass saturation to 1.8'
)
assert.match(
  baseCss,
  /--glass-thick-saturation:\s*1\.8;/,
  'base.css must boost thick glass saturation to 1.8'
)
assert.match(
  baseCss,
  /--glass-thin-blur:\s*10px;/,
  'base.css must preserve thin blur for M3 compatibility'
)
assert.match(
  baseCss,
  /--glass-thick-blur:\s*28px;/,
  'base.css must preserve thick blur for M3 compatibility'
)
assert.match(
  baseCss,
  /img\s*\{[^}]*border-radius:\s*var\(--radius-md\)/,
  'img must apply squircle border radius'
)

// 2. Component Apple HIG alignment
const baseBtn = read('src/renderer/src/components/BaseBtn.vue')
assert.match(baseBtn, /border-radius:\s*var\(--form-radius\);/, 'BaseBtn must use form-radius')
assert.match(baseBtn, /border-radius:\s*var\(--radius-sm\);/, 'BaseBtn .min must use radius-sm')

const baseSelect = read('src/renderer/src/components/BaseSelect.vue')
assert.match(
  baseSelect,
  /border-radius:\s*var\(--form-radius\);/,
  'BaseSelect must use form-radius'
)

const detailHeader = read('src/renderer/src/components/DetailHeader.vue')
assert.match(
  detailHeader,
  /border-radius:\s*var\(--radius-card\);/,
  'DetailHeader album cover must use radius-card'
)
assert.match(
  detailHeader,
  /class="fav pressable"/,
  'DetailHeader favorite button must have pressable tactile feedback'
)

const playlistCard = read('src/renderer/src/components/PlaylistCard.vue')
assert.match(
  playlistCard,
  /border-radius:\s*var\(--radius-md\);/,
  'PlaylistCard cover must use radius-md'
)

const playerBar = read('src/renderer/src/components/PlayerBar.vue')
assert.match(
  playerBar,
  /border-radius:\s*var\(--radius-md\);/,
  'PlayerBar cover must use radius-md'
)

const settingsIndex = read('src/renderer/src/views/settings/index.vue')
assert.match(settingsIndex, /margin:\s*var\(--gap-card\)\s*0;/, 'settings cards must use gap-card')
assert.match(
  settingsIndex,
  /padding:\s*var\(--padding-card\)\s*20px;/,
  'settings cards must use padding-card'
)
assert.match(
  settingsIndex,
  /border-radius:\s*var\(--radius-card\);/,
  'settings cards must use radius-card'
)
assert.match(
  settingsIndex,
  /border-radius:\s*var\(--form-radius\);/,
  'settings inputs must use form-radius'
)

// 3. Continuous FPS & GPU Scheduling Contract
const settingsTs = read('src/common/types/settings.ts')
assert.match(
  settingsTs,
  /themeFps:\s*number\s*\|\s*null/,
  'AppSettings.behavior must define themeFps: number | null'
)
assert.match(
  settingsTs,
  /themeFps:\s*null/,
  'DEFAULT_SETTINGS.behavior must default themeFps to null'
)

const settingAnimation = read('src/renderer/src/views/settings/SettingAnimation.vue')
assert.match(
  settingAnimation,
  /极光背景帧率调度/,
  'SettingAnimation must provide FPS scheduling title'
)
assert.match(settingAnimation, /自适应刷新率/, 'SettingAnimation must offer adaptive FPS option')
assert.match(settingAnimation, /静态节能/, 'SettingAnimation must offer static power-saving option')
assert.match(settingAnimation, /自定义上限/, 'SettingAnimation must offer custom FPS option')
assert.match(
  settingAnimation,
  /type="range"[\s\S]*min="15"[\s\S]*max="144"/,
  'SettingAnimation must provide continuous 15..144 FPS slider'
)

const amllBg = read('src/renderer/src/components/AmllBackground.vue')
assert.match(
  amllBg,
  /const requestedFps = settingsStore\.settings\.behavior\.themeFps \?\? null/,
  'AmllBackground must read requestedFps from store'
)
assert.match(
  amllBg,
  /watch\(\s*\(\)\s*=>\s*settingsStore\.settings\.behavior\.themeFps/,
  'AmllBackground must watch themeFps changes'
)

const applyTs = read('src/renderer/src/theme/apply.ts')
assert.match(applyTs, /export function setThemeFps/, 'apply.ts must export setThemeFps')
assert.match(
  applyTs,
  /motionProfile\(themeId,\s*reducedMotionQuery\.matches,\s*currentThemeFps\)/,
  'apply.ts must pass currentThemeFps to native profile'
)

console.log('M38 Apple Design HIG & stepless FPS GPU scheduling tests passed')
