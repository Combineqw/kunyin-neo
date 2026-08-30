import { spawnSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)))
const source = join(projectRoot, 'scripts', 'theme-contrast-tests.ts')
const loader = join(projectRoot, 'scripts', 'typescript-test-loader.mjs')
const result = spawnSync(
  process.execPath,
  ['--experimental-strip-types', '--experimental-loader', pathToFileURL(loader).href, source],
  { cwd: projectRoot, stdio: 'inherit' }
)
process.exit(result.status ?? 1)
