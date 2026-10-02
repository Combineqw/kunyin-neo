import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const source = readFileSync(
  new URL('../src/renderer/src/components/AmllBackground.vue', import.meta.url),
  'utf8'
)

assert.match(
  source,
  /theme\s*\.\s*motionProfile\(/,
  'renderer must request the Rust motion profile'
)
assert.match(
  source,
  /setStaticMode\(gpuUnavailable \|\| !profile\.enabled\)/,
  'profile and GPU state must control static mode'
)
assert.match(source, /onGpuStatus/, 'renderer must subscribe to GPU health changes')
assert.match(source, /gpuStatus\(\)/, 'renderer must read initial GPU health')
assert.match(source, /gpuUnavailable \|\| !profile\.enabled/, 'GPU failure must force static mode')
assert.match(source, /setFPS\(profile\.targetFps\)/, 'native target FPS must control renderer FPS')
assert.match(source, /detectDisplayRefreshHz/, 'renderer must provide a finite display refresh hint')
assert.match(source, /displayRefreshHz/, 'display refresh hint must reach the native profile')
assert.match(source, /MutationObserver/, 'theme changes must refresh the motion profile')
assert.match(source, /prefers-reduced-motion/, 'reduced-motion changes must refresh the profile')
assert.doesNotMatch(
  source,
  /r\.setFPS\(30\)/,
  'renderer must not use a fixed FPS before the profile resolves'
)
assert.doesNotMatch(source, /FALLBACK_FPS/, 'compatibility layer must not define a fixed FPS fallback')

console.log('M34 theme renderer wiring tests passed')
