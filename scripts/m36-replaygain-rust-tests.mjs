import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M36 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

assert.equal(typeof native.audioReplayGainDb, 'function')
assert.equal(typeof native.audioReplayGainLinear, 'function')
assert.equal(native.audioReplayGainDb(-7.5, null, 1.2, null, false, 2, 6), -5.5)
assert.equal(native.audioReplayGainDb(4, null, 1, null, false, 0, 6), 0)
assert.equal(native.audioReplayGainDb(null, -4, null, null, true, 0, 6), -4)
assert.equal(native.audioReplayGainDb(null, null, null, null, false, 0, 6), 0)
assert.equal(native.audioReplayGainDb(-60, null, null, null, false, 0, 6), -18)
assert.ok(Math.abs(native.audioReplayGainLinear(0) - 1) < 1e-12)
assert.ok(Math.abs(native.audioReplayGainLinear(6) - 1.9952623149688795) < 1e-12)
console.log('M36 Rust ReplayGain DSP boundary smoke test passed')
