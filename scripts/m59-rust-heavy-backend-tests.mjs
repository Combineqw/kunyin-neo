import assert from 'node:assert/strict'
import { writeFileSync, unlinkSync, mkdirSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import {
  nativeMatcherNormalizeMusicText,
  nativeMatcherArtistsMatch,
  nativeMatcherSelectEnrichmentCandidate,
  nativeMatcherScoreMusicMatch,
  nativeMatcherRankCandidates,
  nativeDownloadSniffAudioExtension,
  nativeDownloadParseFlacQualityTag,
  nativeDownloadNormalizeTitle,
  nativeDownloadSelectAlternativeCandidate
} from '../src/main/native/bridge-runtime.js'
import {
  normalizeMusicText,
  artistsMatch,
  selectEnrichmentCandidate,
  calculateMatchScore,
  rankEnrichmentCandidates
} from '../src/main/modules/local-music/match.ts'

console.log('Running M59 Heavy Backend Rust Migration Verification Tests...')

// 1. Verify text normalisation algorithm in Rust
console.log('1. 验证 Rust 原生高性能文本规范化与 FT/FEAT 清洗...')
const testCases = [
  ['Jay Chou feat. JJ Lin', 'jaychoujjlin'],
  ['Taylor Swift ft. Ice Spice', 'taylorswifticespice'],
  ['陈奕迅 - 富士山下', '陈奕迅富士山下'],
  ['feature film', 'featurefilm'],
  ['（全角） ＡＢＣ １２３', '全角abc123']
]

for (const [input, expected] of testCases) {
  const rustNorm = nativeMatcherNormalizeMusicText(input)
  const tsNorm = normalizeMusicText(input)
  assert.equal(rustNorm, expected, `Rust normalizer should match expected for "${input}"`)
  assert.equal(tsNorm, expected, `TypeScript match.ts wrapper should match expected for "${input}"`)
}

// 2. Verify artists match algorithm in Rust
console.log('2. 验证 Rust 原生艺术家包含与多重归属判定...')
assert.equal(nativeMatcherArtistsMatch('周杰伦', '周杰伦'), true)
assert.equal(nativeMatcherArtistsMatch('周杰伦 feat. 蔡依林', '周杰伦'), true)
assert.equal(nativeMatcherArtistsMatch('林俊杰', '林俊杰 / 阿信'), true)
assert.equal(nativeMatcherArtistsMatch('周杰伦', '陈奕迅'), false)
assert.equal(artistsMatch('周杰伦', '周杰伦'), true)
assert.equal(artistsMatch('周杰伦', '陈奕迅'), false)

// 3. Verify enrichment candidate selection with ambiguity resolution in Rust
console.log('3. 验证 Rust 原生富化候选匹配与歧义仲裁...')
const tags = { title: '晴天', artist: '周杰伦' }
const candidates = [
  {
    type: 'wy',
    id: 1001,
    title: '晴天',
    artist: '周杰伦',
    duration: 269000,
    cover: 'https://p1.music.126.net/cover.jpg',
    album: '叶惠美'
  },
  {
    type: 'wy',
    id: 1002,
    title: '晴天 (Live)',
    artist: '周杰伦',
    duration: 270000,
    cover: 'https://p1.music.126.net/cover2.jpg',
    album: '超时代演唱会'
  }
]

const selected = selectEnrichmentCandidate(tags, 269500, candidates)
assert.ok(selected, 'Must select exact unambiguous candidate')
assert.equal(selected.id, 1001)

// Ambiguous candidates -> returns null
const ambiguousCandidates = [
  candidates[0],
  {
    type: 'qq',
    id: 1003,
    title: '晴天',
    artist: '周杰伦',
    duration: 269200
  }
]
const ambiguousSelected = selectEnrichmentCandidate(tags, 269000, ambiguousCandidates)
assert.equal(ambiguousSelected, null, 'Must reject multiple ambiguous candidates')

// Duration exceeding tolerance (>5000ms) -> returns null
const durationMismatch = selectEnrichmentCandidate(tags, 280000, [candidates[0]])
assert.equal(durationMismatch, null, 'Must reject duration delta > 5000ms')

// 4. Verify fuzzy match scoring & ranking in Rust
console.log('4. 验证 Rust 原生模糊匹配评分与候选优选排序...')
const scoreExact = calculateMatchScore(tags, 269000, candidates[0])
assert.ok(scoreExact > 0.9, `Exact match score should be > 0.9, got ${scoreExact}`)

const scoreLive = calculateMatchScore(tags, 269000, candidates[1])
assert.ok(scoreLive < scoreExact, 'Live version score should be strictly lower than exact match')
assert.ok(scoreLive > 0.4, 'Live version with same artist should retain moderate score')

const ranked = rankEnrichmentCandidates(tags, 269000, [
  { type: 'kg', id: 3, title: '稻香', artist: '周杰伦', duration: 240000 },
  candidates[1],
  candidates[0]
])
assert.equal(ranked.length, 3)
assert.equal(ranked[0].item.id, 1001, 'Top candidate must be exact match')
assert.equal(ranked[1].item.id, 1002, 'Second candidate must be live match')
assert.equal(ranked[2].item.id, 3, 'Last candidate must be unrelated song')

// 5. Verify audio extension sniffing in Rust
console.log('5. 验证 Rust 原生音频容器头嗅探 (OggS, fLaC, ID3, MP3)...')
const testDir = join(tmpdir(), `kunyin-m59-test-${Date.now()}`)
if (!existsSync(testDir)) mkdirSync(testDir, { recursive: true })

const oggPath = join(testDir, 'test.ogg.tmp')
writeFileSync(oggPath, Buffer.from([0x4f, 0x67, 0x67, 0x53, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]))
assert.equal(nativeDownloadSniffAudioExtension(oggPath), '.ogg')

const flacPath = join(testDir, 'test.flac.tmp')
writeFileSync(flacPath, Buffer.from([0x66, 0x4c, 0x61, 0x43, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00, 0x00, 0x00]))
assert.equal(nativeDownloadSniffAudioExtension(flacPath), '.flac')

const mp3Path = join(testDir, 'test.mp3.tmp')
writeFileSync(mp3Path, Buffer.from([0x49, 0x44, 0x33, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]))
assert.equal(nativeDownloadSniffAudioExtension(mp3Path), '.mp3')

const invalidPath = join(testDir, 'test.bin')
writeFileSync(invalidPath, Buffer.from([0x00, 0x01, 0x02, 0x03]))
assert.equal(nativeDownloadSniffAudioExtension(invalidPath), null)

// 6. Verify FLAC STREAMINFO tag parsing in Rust
console.log('6. 验证 Rust 原生 FLAC STREAMINFO 采样率与位深解析...')
const flacHeader = Buffer.alloc(42)
flacHeader[0] = 0x66 // f
flacHeader[1] = 0x4c // L
flacHeader[2] = 0x61 // a
flacHeader[3] = 0x43 // C
// 44100 Hz (0xAC44), 16 Bit:
flacHeader[18] = 10
flacHeader[19] = 196
flacHeader[20] = (4 << 4) | 0x00
flacHeader[21] = 0xf0
const flacStreaminfoPath = join(testDir, 'streaminfo.flac')
writeFileSync(flacStreaminfoPath, flacHeader)
assert.equal(nativeDownloadParseFlacQualityTag(flacStreaminfoPath), '[16Bit-44.1kHz]')

// 7. Verify title normalization & alternative source matching in Rust
console.log('7. 验证 Rust 原生换源标题归一化与候选首中提取...')
assert.equal(nativeDownloadNormalizeTitle('七里香 (Live) - 周杰伦'), '七里香live周杰伦')

const altCandidates = [
  { id: 1, title: '东风破', artist: '周杰伦' },
  { id: 2, title: '七里香', artist: '周杰伦' },
  { id: 3, title: '晴天', artist: '周杰伦' }
]
const matchedAlt = nativeDownloadSelectAlternativeCandidate('七里香', altCandidates)
assert.ok(matchedAlt, 'Should find matching alternative song')
assert.equal(matchedAlt.id, 2)

// Clean up temporary files
try {
  unlinkSync(oggPath)
  unlinkSync(flacPath)
  unlinkSync(mp3Path)
  unlinkSync(invalidPath)
  unlinkSync(flacStreaminfoPath)
} catch {
  // ignore cleanup error
}

console.log('All M59 Heavy Backend Rust Migration Verification Tests passed successfully!')
