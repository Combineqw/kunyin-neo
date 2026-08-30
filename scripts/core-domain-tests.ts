import assert from 'node:assert/strict'
import {
  parseEqualizerApoText,
  parseExternalEqualizerText,
  parseTenBandGainList,
  serializeEqualizerApoText
} from '../src/common/domain/equalizerText'
import { readIrsWaveInfo } from '../src/common/domain/irs'
import { createDailyRecommendations, createHeartbeatRecommendations, getDailyRecommendationScore } from '../src/common/domain/recommendation'
import type { RecommendationSong } from '../src/common/types/recommendation'
import type { LocalMusicItem } from '../src/common/types/music'

function makeSong(id: number, overrides: Partial<RecommendationSong> = {}): RecommendationSong {
  const item: LocalMusicItem = {
    type: 'local',
    id,
    title: `歌曲 ${id}`,
    artist: `歌手 ${id}`,
    album: '测试专辑',
    cover: '',
    duration: 240000,
    qualities: {},
    filePath: `C:/music/${id}.mp3`
  }
  return {
    item,
    source: 'local',
    songId: id,
    tags: [],
    playCount: 0,
    completionCount: 0,
    skipCount: 0,
    completionRate: 0,
    isFavorite: false,
    ...overrides,
    item: { ...item, ...(overrides.item ?? {}) }
  }
}

function testDailyRecommendation(): void {
  const favorite = makeSong(1, { isFavorite: true, item: { ...makeSong(1).item, artist: '同一歌手' } })
  const completed = makeSong(2, { completionRate: 1, playCount: 8, item: { ...makeSong(2).item, artist: '同一歌手' } })
  const skipped = makeSong(3, { skipCount: 6 })
  const cold = makeSong(4, { genre: '流行' })
  const extra = makeSong(5, { genre: '摇滚' })
  assert.ok(getDailyRecommendationScore(favorite, new Set()) > getDailyRecommendationScore(completed, new Set()))
  assert.ok(getDailyRecommendationScore(completed, new Set()) > getDailyRecommendationScore(skipped, new Set()))
  assert.ok(getDailyRecommendationScore(cold, new Set(['local_4'])) < getDailyRecommendationScore(cold, new Set()))

  const output = createDailyRecommendations([favorite, completed, skipped, cold, extra], {
    limit: 4,
    maxPerArtist: 1,
    random: () => 0
  })
  assert.equal(output.length, 4)
  assert.equal(output.filter((entry) => entry.song.item.artist === '同一歌手').length, 1)
  assert.equal(output[0].song.songId, 1)
}

function testHeartbeatRecommendation(): void {
  const seed = makeSong(100, { item: { ...makeSong(100).item, artist: '种子歌手' } })
  const familiar = Array.from({ length: 7 }, (_, index) =>
    makeSong(index + 1, { isFavorite: true, item: { ...makeSong(index + 1).item, artist: '种子歌手' } })
  )
  const discovery = Array.from({ length: 3 }, (_, index) =>
    makeSong(index + 20, { item: { ...makeSong(index + 20).item, artist: '种子歌手' } })
  )
  const output = createHeartbeatRecommendations(seed, [...familiar, ...discovery], {
    limit: 10,
    familiarRatio: 0.7,
    random: () => 0
  })
  assert.equal(output.length, 10)
  assert.equal(output.slice(0, 7).filter((entry) => entry.familiar).length, 7)
  assert.equal(output.slice(7).filter((entry) => !entry.familiar).length, 3)
}

function makeWaveFile({
  sampleRate = 48000,
  channels = 2,
  bitsPerSample = 16,
  dataBytes = 960
}: {
  sampleRate?: number
  channels?: number
  bitsPerSample?: number
  dataBytes?: number
} = {}): Uint8Array {
  const blockAlign = channels * (bitsPerSample / 8)
  const byteRate = sampleRate * blockAlign
  const fmtSize = 16
  const riffSize = 4 + 8 + fmtSize + 8 + dataBytes
  const bytes = new Uint8Array(8 + riffSize)
  const view = new DataView(bytes.buffer)
  const writeId = (offset: number, value: string): void => {
    for (let index = 0; index < value.length; index++) bytes[offset + index] = value.charCodeAt(index)
  }
  writeId(0, 'RIFF')
  view.setUint32(4, riffSize, true)
  writeId(8, 'WAVE')
  writeId(12, 'fmt ')
  view.setUint32(16, fmtSize, true)
  view.setUint16(20, 1, true)
  view.setUint16(22, channels, true)
  view.setUint32(24, sampleRate, true)
  view.setUint32(28, byteRate, true)
  view.setUint16(32, blockAlign, true)
  view.setUint16(34, bitsPerSample, true)
  writeId(36, 'data')
  view.setUint32(40, dataBytes, true)
  return bytes
}

function testIrsWaveInfo(): void {
  const info = readIrsWaveInfo(makeWaveFile({ sampleRate: 44100, channels: 1, dataBytes: 88200 }))
  assert.equal(info.sampleRate, 44100)
  assert.equal(info.channels, 1)
  assert.equal(info.bitsPerSample, 16)
  assert.equal(info.durationMs, 1000)
  assert.throws(() => readIrsWaveInfo(new Uint8Array([1, 2, 3])), /RIFF\/WAVE/)
  assert.throws(() => readIrsWaveInfo(makeWaveFile({ dataBytes: 1000 }).subarray(0, 20)), /chunk 长度/)
}

function testEqualizerText(): void {
  const profile = parseEqualizerApoText(
    ['# AutoEq', 'Preamp: -5.25 dB', 'Filter 1: ON PK Fc 100 Hz Gain 3 dB Q 1.41', 'Filter 2: OFF LSC Fc 80 Hz Gain 2.5 dB', 'Filter 3: ON HSC Fc 8000 Hz Gain -2 dB Q 0.7'].join('\n'),
    'headphones.txt'
  )
  assert.equal(profile.name, 'headphones')
  assert.equal(profile.preampDb, -5.25)
  assert.equal(profile.filters?.length, 3)
  assert.deepEqual(profile.filters?.[0], { type: 'peaking', frequency: 100, gainDb: 3, q: 1.41, enabled: true })
  assert.deepEqual(profile.filters?.[1], { type: 'lowshelf', frequency: 80, gainDb: 2.5, enabled: false })
  const exported = serializeEqualizerApoText(profile)
  assert.match(exported, /Preamp: -5.25 dB/)
  assert.match(exported, /Filter 3: ON HSC Fc 8000 Hz Gain -2 dB Q 0.7/)
  const roundTrip = parseEqualizerApoText(exported, 'roundtrip.txt')
  assert.equal(roundTrip.preampDb, -5.25)
  assert.deepEqual(roundTrip.filters, profile.filters)

  const tenBand = parseTenBandGainList('0, 1, 2, 3, 4, 5, 6, 7, 8, 9', 'ten.txt')
  assert.equal(tenBand.bands.length, 10)
  assert.equal(tenBand.bands[9].gainDb, 9)
  const graphicEq = parseExternalEqualizerText(
    'GraphicEQ: 20 -4.7; 31 -5.4; 62 -6.0; 125 -7.2; 250 -6.7; 500 -3.8; 1000 -2.2; 2000 -3.0; 4000 -0.8; 8000 -1.7; 16000 -8.0; 19871 -13.0',
    'Moondrop Quarks GraphicEq.txt'
  )
  assert.equal(graphicEq.name, 'Moondrop Quarks GraphicEq')
  assert.equal(graphicEq.bands.length, 10)
  assert.equal(graphicEq.filters?.length ?? 0, 0)
  assert.ok(Math.abs(graphicEq.bands[0].gainDb + 5.4) < 0.01)
  assert.throws(() => parseEqualizerApoText('Filter 1: ON HP Fc 50 Hz Gain 1 dB', 'bad.txt'), /第 1 行/)
}

testDailyRecommendation()
testHeartbeatRecommendation()
testEqualizerText()
testIrsWaveInfo()
console.log('核心推荐、EQ 与 IRS 解析测试通过')
