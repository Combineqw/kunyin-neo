/**
 * Compare the QQ and Netease provider TypeScript parsers with the optional
 * Rust parsers.
 * The input corpus is deliberately small and deterministic: it exercises the
 * accepted DTO shape, missing required fields, quality boundaries, and QQC's
 * shared `type: qq` output contract without making a network request.
 */
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const require = createRequire(import.meta.url)
const { parseTrackInfo: parseQqTrackInfo } = await import('../src/main/providers/qq/item.ts')
const { parseTrackInfo: parseWyTrackInfo } = await import('../src/main/providers/wy/item.ts')

const nativeDir = join(process.cwd(), 'crates', 'aurora-native')
const nativeFile = readdirSync(nativeDir).find((file) => file.endsWith('.node'))
if (!nativeFile || !existsSync(join(nativeDir, nativeFile))) {
  throw new Error('aurora-native .node 不存在，请先运行 npm run native:build')
}
const native = require(join(nativeDir, nativeFile))
assert.equal(typeof native.providerParseQqTrack, 'function')
assert.equal(typeof native.providerParseWyTrack, 'function')

const qqCorpus = [
  {
    id: 12,
    mid: 'song-mid',
    title: 'Song',
    interval: 231,
    singer: [{ id: 7, name: 'Singer', mid: 'singer-mid' }],
    album: { id: 99, name: 'Album', mid: 'album-mid' },
    file: {
      media_mid: 'media',
      size_128mp3: 100,
      size_320mp3: 200,
      size_flac: 500,
      size_hires: 800,
      size_new: [1, 2, 3]
    },
    vs: ['', '', '', 'master', 'atmos'],
    mv: { vid: 'mv-id' }
  },
  {
    id: '13',
    name: 'Fallback title',
    singer: [{ name: '甲' }, { name: '' }, { id: 3, mid: 'ignored' }],
    album: { id: 0, name: 'Album', mid: 'album-mid' },
    file: { media_mid: 'media', size_new: ['4', 0, 0] },
    vs: ['', '', '', 'master']
  },
  { id: 1, title: 'Missing file' },
  { id: 0, title: 'Invalid id', file: {} },
  { id: 2, title: '', file: {} }
]

let qqPassed = 0
for (const item of qqCorpus) {
  // N-API crosses a JSON boundary; JSON.stringify is the contract that
  // removes the TypeScript parser's own `undefined` optional properties.
  const nodeValue = JSON.parse(JSON.stringify(parseQqTrackInfo(item)))
  const rustRaw = native.providerParseQqTrack(JSON.stringify(item))
  const rustValue = JSON.parse(rustRaw)
  assert.deepEqual(rustValue, nodeValue)
  qqPassed += 1
}

const wyCorpus = [
  {
    id: '21',
    name: 'Cloud Song',
    dt: 231000,
    ar: [{ id: 7, name: 'Singer', picUrl: 'pic' }, { id: 8, name: '' }],
    al: { id: 0, name: 'Album', picUrl: 'cover' },
    l: { size: 100 },
    h: { size: '200' },
    sq: { size: 500 },
    hr: { size: 800 },
    mv: '042'
  },
  { id: 0, name: 'Zero id', ar: [], al: { id: null, name: 'Album' }, mv: 0 },
  { id: -1, name: 'Invalid id' },
  { id: 1, name: '' },
  { id: 2, name: 'Unknown artist', al: {}, mv: '0', l: { size: -1 } }
]

let wyPassed = 0
for (const item of wyCorpus) {
  const nodeValue = JSON.parse(JSON.stringify(parseWyTrackInfo(item)))
  const rustRaw = native.providerParseWyTrack(JSON.stringify(item))
  const rustValue = JSON.parse(rustRaw)
  assert.deepEqual(rustValue, nodeValue)
  wyPassed += 1
}

console.log(`Provider Rust 影子比对通过：QQ ${qqPassed}/${qqCorpus.length}，网易云 ${wyPassed}/${wyCorpus.length}`)
