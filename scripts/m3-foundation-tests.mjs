/** M3 地基批的静态接线检查。 */
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const read = (path) => readFileSync(join(root, path), 'utf8')
const checks = [
  [
    'src/renderer/src/App.vue',
    ["window.addEventListener('blur', syncWindowActivity)", "document.addEventListener('visibilitychange', syncWindowActivity)"]
  ],
  [
    'src/renderer/src/components/AmllBackground.vue',
    ['renderer.pause()', 'renderer.resume()', "document.addEventListener('visibilitychange', syncRendererActivity)"]
  ],
  [
    'src/renderer/src/assets/base.css',
    ['html.window-inactive .aurora-band', 'animation-play-state: paused', '--glass-thin-blur: 10px', '--glass-thick-blur: 28px']
  ],
  [
    'src/renderer/src/theme/coverPalette.ts',
    ['64', 'QuantizerCelebi', 'Score.score', 'Hct.from', 'COVER_ACCENT_FALLBACK']
  ]
]

const failures = []
for (const [file, needles] of checks) {
  const source = read(file)
  for (const needle of needles) {
    if (!source.includes(needle)) failures.push(`${file} 缺少 ${needle}`)
  }
}

if (failures.length) {
  console.error(`M3 地基接线检查失败（${failures.length} 项）：`)
  for (const failure of failures) console.error(`  ✗ ${failure}`)
  process.exit(1)
}
console.log('M3 地基接线检查通过：失焦暂停、低负载玻璃 tokens、封面 HCT 取色均已接线')
