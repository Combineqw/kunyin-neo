import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const settings = readFileSync(new URL('../src/common/types/settings.ts', import.meta.url), 'utf8')
const ipc = readFileSync(new URL('../src/common/types/ipc.ts', import.meta.url), 'utf8')
const lyricWindow = readFileSync(
  new URL('../src/main/modules/desktop-lyrics/window.ts', import.meta.url),
  'utf8'
)
const miniWindow = readFileSync(new URL('../src/main/modules/mini-player/window.ts', import.meta.url), 'utf8')
const view = readFileSync(new URL('../src/renderer/src/desktop-lyrics/DesktopLyrics.vue', import.meta.url), 'utf8')
const bridge = readFileSync(
  new URL('../src/renderer/src/window-bridges/useDesktopLyricBridge.ts', import.meta.url),
  'utf8'
)

assert.match(settings, /desktopMode:\s*'lyrics' \| 'combined'/)
assert.match(settings, /desktopMode: 'lyrics'/)
assert.match(ipc, /DESKTOP_LYRIC_COMMAND/)
assert.match(ipc, /cover\?: string/)
assert.match(lyricWindow, /DESKTOP_LYRIC_COMMAND/)
assert.match(miniWindow, /desktopMode === 'combined'/)
assert.match(miniWindow, /player: \{ miniPlayerEnabled: false \}/)
assert.match(view, /v-if="combined" class="dl-track"/)
assert.match(
  readFileSync(new URL('../src/renderer/src/views/settings/SettingDesktopLyric.vue', import.meta.url), 'utf8'),
  /setting_dl_mode/
)
assert.match(view, /window\.api\.desktopLyric\.command\(command\)/)
assert.match(view, /toggleFavorite/)
assert.match(bridge, /cover: c\?\.cover/)
assert.match(bridge, /duration: duration\.value/)

console.log('M35 combined desktop lyric window contract passed')
