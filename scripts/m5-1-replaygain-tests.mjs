import assert from 'node:assert/strict'

const { normalizeReplayGain, resolveReplayGainDb } =
  await import('../src/common/domain/replayGain.ts')

assert.deepEqual(normalizeReplayGain({ trackDb: -7.5, trackPeak: 1.2, ignored: 1 }), {
  trackDb: -7.5,
  trackPeak: 1.2
})
assert.equal(resolveReplayGainDb({ trackDb: 4, trackPeak: 1 }, 'track', 0, 6), 0)
assert.equal(resolveReplayGainDb({ trackDb: -7.5 }, 'track', 2, 6), -5.5)
assert.equal(resolveReplayGainDb({ albumDb: -4 }, 'track', 0, 6), -4)
assert.equal(resolveReplayGainDb(undefined, 'album'), 0)

console.log('M5.1 ReplayGain 纯逻辑检查通过')
