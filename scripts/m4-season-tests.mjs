import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'

const { getAuroraSeason, getSeasonalAuroraThemeId, SEASONAL_AURORA_AUTO_ID } =
  await import('../src/renderer/src/theme/seasons.ts')

assert.equal(getAuroraSeason(new Date(2026, 2, 1)), 'spring')
assert.equal(getAuroraSeason(new Date(2026, 5, 1)), 'summer')
assert.equal(getAuroraSeason(new Date(2026, 8, 1)), 'autumn')
assert.equal(getAuroraSeason(new Date(2026, 11, 1)), 'winter')
assert.equal(getAuroraSeason(new Date(2026, 0, 1)), 'winter')
assert.equal(getSeasonalAuroraThemeId(new Date(2026, 6, 1)), 'aurora_summer')
assert.equal(SEASONAL_AURORA_AUTO_ID, 'aurora_seasonal_auto')

const themes = readFileSync(join(process.cwd(), 'src/renderer/src/theme/themes.ts'), 'utf8')
for (const id of ['aurora_spring', 'aurora_summer', 'aurora_autumn', 'aurora_winter']) {
  assert.match(themes, new RegExp(`id: '${id}'`))
}
const css = readFileSync(join(process.cwd(), 'src/renderer/src/assets/base.css'), 'utf8')
assert.match(css, /--anim-dur-theme:\s*0\.28s/)
assert.match(css, /html\.theme-transition[\s\S]*var\(--anim-dur-theme\)/)
console.log('M4 四季映射检查通过：四季边界与自动主题 ID 正确')
