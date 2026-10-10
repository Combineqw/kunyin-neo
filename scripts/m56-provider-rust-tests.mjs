import assert from 'node:assert/strict'
import {
  nativeParseKgTrack,
  nativeParseKwTrack,
  nativeKgSign
} from '../src/main/native/bridge-runtime.js'
import { kgSign } from '../src/main/providers/kg/sign.ts'
import { parseSearchItem as tsParseKg } from '../src/main/providers/kg/item.ts'
import { parseSearchItem as tsParseKw } from '../src/main/providers/kw/item.ts'

console.log('Running M56 Multi-Source Provider Rust Downsink & SearchView Verification Tests...')

// 1. Verify KuGou MD5 request signature calculation in Rust
console.log('1. 验证 KuGou 原生 MD5 签名计算与 TypeScript 算法一致性...')
const params = {
  appid: '1005',
  clientver: '114514',
  mid: '888888',
  keyword: '周杰伦',
  page: '1',
  pagesize: '30'
}
const tsSignature = kgSign(params, '')
const rustSignature = nativeKgSign(params, '')
assert.ok(rustSignature, 'nativeKgSign must return a non-null 32-character hex string')
assert.equal(rustSignature.length, 32, 'MD5 signature must be 32 characters')
assert.equal(rustSignature, tsSignature, 'Rust native signature must 100% match TypeScript reference')

// 2. Verify KuGou raw song item parsing in Rust
console.log('2. 验证 KuGou 歌曲元数据与音质结构原生解析...')
const kgRawItem = {
  FileHash: 'F1234567890ABCDEF1234567890ABCDEF',
  FileSize: 4194304,
  HQFileHash: 'H1234567890ABCDEF1234567890ABCDEF',
  HQFileSize: 10485760,
  SQFileHash: 'S1234567890ABCDEF1234567890ABCDEF',
  SQFileSize: 33554432,
  ResFileHash: 'R1234567890ABCDEF1234567890ABCDEF',
  ResFileSize: 67108864,
  SongName: '晴天',
  Suffix: ' (现场版)',
  SingerName: '周杰伦',
  AlbumName: '叶惠美',
  AlbumID: 998,
  Duration: 269,
  MixSongID: '668822',
  Singers: [{ name: '周杰伦', id: 77 }]
}

const rustKgTrack = nativeParseKgTrack(kgRawItem)
assert.ok(rustKgTrack, 'nativeParseKgTrack must successfully parse valid KG payload')
assert.equal(rustKgTrack.type, 'kg')
assert.equal(rustKgTrack.id, 668822)
assert.equal(rustKgTrack.title, '晴天 (现场版)')
assert.equal(rustKgTrack.artist, '周杰伦')
assert.equal(rustKgTrack.album, '叶惠美')
assert.equal(rustKgTrack.duration, 269000)
assert.equal(rustKgTrack.qualities['128k'].filesize, 4194304)
assert.equal(rustKgTrack.qualities['320k'].filesize, 10485760)
assert.equal(rustKgTrack.qualities['flac'].filesize, 33554432)
assert.equal(rustKgTrack.qualities['hires'].filesize, 67108864)
assert.equal(rustKgTrack.singers[0].name, '周杰伦')
assert.equal(rustKgTrack.singers[0].singerId, 77)

const tsKgTrack = tsParseKg(kgRawItem)
assert.deepEqual(rustKgTrack, tsKgTrack, 'Rust KG parser output must align with TS parser contract')

// 3. Verify KuWo raw song item parsing in Rust
console.log('3. 验证 KuWo 歌曲 minfo 级联音质与元数据原生解析...')
const kwRawItem = {
  MUSICRID: 'MUSIC_154988',
  SONGNAME: '夜曲',
  ARTIST: '周杰伦',
  ALBUM: '十一月的萧邦',
  ALBUMID: '776',
  DURATION: 226,
  N_MINFO: 'level:ff,bitrate:2000,format:flac,size:28.5MB;level:p,bitrate:320,format:mp3,size:8.5MB;level:df,bitrate:128,format:mp3,size:3.5MB',
  allartistid: '77',
  HASMV: '1'
}

const rustKwTrack = nativeParseKwTrack(kwRawItem)
assert.ok(rustKwTrack, 'nativeParseKwTrack must successfully parse valid KW payload')
assert.equal(rustKwTrack.type, 'kw')
assert.equal(rustKwTrack.id, 154988)
assert.equal(rustKwTrack.title, '夜曲')
assert.equal(rustKwTrack.artist, '周杰伦')
assert.equal(rustKwTrack.album, '十一月的萧邦')
assert.equal(rustKwTrack.duration, 226000)
assert.equal(rustKwTrack.qualities['flac'].bitrate, 2000)
assert.equal(rustKwTrack.qualities['320k'].bitrate, 320)
assert.equal(rustKwTrack.qualities['128k'].bitrate, 128)
assert.equal(rustKwTrack.singers[0].singerId, 77)
assert.equal(rustKwTrack.mvid, '154988')

const tsKwTrack = tsParseKw(kwRawItem)
assert.deepEqual(rustKwTrack, tsKwTrack, 'Rust KW parser output must align with TS parser contract')

console.log('All M56 Multi-Source Provider Rust Downsink & SearchView Verification Tests passed successfully!')
