import assert from 'node:assert/strict'

const { normalizeMusicText, selectEnrichmentCandidate } =
  await import('../src/main/modules/local-music/match.ts')

const local = (overrides = {}) => ({
  type: 'local',
  id: 1,
  title: 'Tagged title',
  artist: 'Tagged artist',
  album: '',
  cover: '',
  duration: 180000,
  qualities: {},
  filePath: 'C:/music/tagged.mp3',
  ...overrides
})

assert.equal(normalizeMusicText('  Hello (feat. A)  '), 'helloa')
const candidate = {
  type: 'wy',
  id: 9,
  title: 'Tagged Title',
  artist: 'Tagged Artist',
  album: 'Remote Album',
  cover: 'https://example.test/cover.jpg',
  duration: 180200,
  qualities: {}
}
assert.equal(selectEnrichmentCandidate(local(), 180000, [candidate]).id, 9)
assert.equal(
  selectEnrichmentCandidate(local(), 180000, [candidate, { ...candidate, id: 10 }]),
  null
)
assert.equal(selectEnrichmentCandidate({ title: 'Filename only', artist: '' }, 180000, [candidate]), null)

console.log('M5 F1 本地信息补全纯逻辑检查通过')
