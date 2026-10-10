import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'

const repoRoot = path.resolve(import.meta.dirname, '..')

function read(relPath) {
  return fs.readFileSync(path.join(repoRoot, relPath), 'utf-8')
}

console.log('Running Apple Design UI Refinements Verification Tests...')

// 1. base.css tokens & typography verification
const baseCss = read('src/renderer/src/assets/base.css')
assert(baseCss.includes('-moz-osx-font-smoothing: grayscale'), 'base.css must include grayscale font smoothing')
assert(baseCss.includes('text-rendering: optimizeLegibility'), 'base.css must include optimizeLegibility')
assert(baseCss.includes('letter-spacing: -0.01em'), 'base.css body must have Apple SF Pro letter spacing')
assert(baseCss.includes('box-shadow: inset 0 0.5px 0 0 var(--glass-highlight)'), 'glass-surface must include Apple specular highlight')
assert(baseCss.includes('--glass-thin-blur: 10px'), 'glass thin blur must be present')
assert(baseCss.includes('--glass-thick-blur: 28px'), 'glass thick blur must be present')
assert(baseCss.includes('letter-spacing: -0.015em'), 'page-title must include title letter spacing')

// 2. MainLayout.vue layout grid & surface verification
const mainLayout = read('src/renderer/src/layouts/MainLayout.vue')
assert(mainLayout.includes('gap: 12px;'), 'shell must use 12px gap adhering to Apple 4pt grid')
assert(mainLayout.includes('padding: 12px;'), 'shell must use 12px padding')
assert(mainLayout.includes('border-radius: 20px;'), 'area-aside must have refined 20px corner radius')

// 3. AppAside.vue navigation indicators
const appAside = read('src/renderer/src/components/AppAside.vue')
assert(appAside.includes('border-radius: 14px;'), 'active indicator must have 14px radius')
assert(appAside.includes('--anim-ease-spring'), 'active indicator must use spring easing')
assert(appAside.includes('calc(100% - 16px)'), 'nav links must have symmetric 8px margins')

// 4. AppToolbar.vue search field & window buttons
const appToolbar = read('src/renderer/src/components/AppToolbar.vue')
assert(appToolbar.includes('backdrop-filter: blur(14px)'), 'search box must have backdrop-filter blur')
assert(appToolbar.includes('letter-spacing: -0.01em'), 'search input must have refined typography')
assert(appToolbar.includes('border-radius: 8px;'), 'window control buttons must have rounded corners')

// 5. PlayerBar.vue playback controls
const playerBar = read('src/renderer/src/components/PlayerBar.vue')
assert(playerBar.includes('width: 44px;'), 'play button must have 44px tactile dimensions')
assert(playerBar.includes('height: 44px;'), 'play button height must match width')
assert(playerBar.includes('border-radius: 10px;'), 'cover must have Apple squircle radius')
assert(playerBar.includes('backdrop-filter: blur(24px) saturate(180%)'), 'volume popover must have Apple frosted glass')

// 6. PlaylistCard.vue card styling
const playlistCard = read('src/renderer/src/components/PlaylistCard.vue')
assert(playlistCard.includes('border-radius: 14px;'), 'playlist cover must have 14px radius')
assert(playlistCard.includes('backdrop-filter: blur(8px)'), 'count badge must have frosted glass')
assert(playlistCard.includes('transform: translateY(-2px)'), 'playlist card hover must have tactile spring lift')

// 7. SongRow.vue row item and tags
const songRow = read('src/renderer/src/components/SongRow.vue')
assert(songRow.includes('border-radius: 10px;'), 'row cover must have 10px radius')
assert(songRow.includes('border-radius: 4px;'), 'quality badges must have pill badge styling')
assert(songRow.includes('border-color: color-mix(in srgb, var(--color-primary) 20%, transparent);'), 'hover border must highlight primary tint')

// 8. DetailHeader.vue action buttons
const detailHeader = read('src/renderer/src/components/DetailHeader.vue')
assert(detailHeader.includes('letter-spacing: -0.02em;'), 'header title must have crisp tracking')
assert(detailHeader.includes('padding: 9px 22px;'), 'play-all button must have Apple capsule padding')

console.log('All Apple Design UI Refinements Verification Tests passed successfully!')
