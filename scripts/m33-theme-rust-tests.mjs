import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M33 smoke test')
const native = require(`../${bindingPath}`)
assert.equal(typeof native.themeMotionProfile, 'function')
const spring = JSON.parse(native.themeMotionProfile('aurora_spring', false))
assert.equal(spring.themeId, 'aurora_spring')
assert.equal(spring.enabled, true)
assert.equal(spring.blobs.length, 5)
assert.equal(spring.frameIntervalMs, 33)
const reduced = JSON.parse(native.themeMotionProfile('aurora_spring', true))
assert.equal(reduced.enabled, false)
assert.equal(reduced.blobs.length, 0)
const plain = JSON.parse(native.themeMotionProfile('green', false))
assert.equal(plain.enabled, false)
console.log('M33 Rust 主题动效 profile smoke test passed')
