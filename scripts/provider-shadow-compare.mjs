/**
 * Compare the QQ provider's TypeScript parser with the optional Rust parser.
 * The input corpus is deliberately small and deterministic: it exercises the
 * accepted DTO shape, missing required fields, quality boundaries, and QQC's
 * shared `type: qq` output contract without making a network request.
 */
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

const require = createRequire(import.meta.url)
const { parseTrackInfo } = await import('../src/main/providers/qq/item.ts')

const nativeDir = join(process.cwd(), 'crates', 'aurora-native')
const nativeFile = readdirSync(nativeDir).find((file) => file.endsWith('.node'))
if (!nativeFile || !existsSync(join(nativeDir, nativeFile))) {
  throw new Error('aurora-native .node 不存在，请先运行 npm run native:build')
}
const native = require(join(nativeDir, nativeFile))
assert.equal(typeof native.providerParseQqTrack, 'function')

const corpus = [
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

let passed = 0
for (const item of corpus) {
  // N-API crosses a JSON boundary; JSON.stringify is the contract that
  // removes the TypeScript parser's own `undefined` optional properties.
  const nodeValue = JSON.parse(JSON.stringify(parseTrackInfo(item)))
  const rustRaw = native.providerParseQqTrack(JSON.stringify(item))
  const rustValue = JSON.parse(rustRaw)
  assert.deepEqual(rustValue, nodeValue)
  passed += 1
}

console.log(`QQ Provider Rust 影子比对通过：${passed}/${corpus.length}`)
