import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import {
  nativeStreamCacheCreate,
  nativeStreamCacheWriteChunk,
  nativeStreamCacheRead,
  nativeStreamCacheStats,
  nativeStreamCacheClose
} from '../src/main/native/bridge-runtime.js'

const bindingPath = 'crates/aurora-native/aurora-native.win32-x64-msvc.node'
assert.ok(existsSync(bindingPath), 'native binding must be built before M40 smoke test')
const require = createRequire(import.meta.url)
const native = require(`../${bindingPath}`)

// 1. Direct native export presence
assert.equal(typeof native.streamCacheCreate, 'function')
assert.equal(typeof native.streamCacheWriteChunk, 'function')
assert.equal(typeof native.streamCacheRead, 'function')
assert.equal(typeof native.streamCacheStats, 'function')
assert.equal(typeof native.streamCacheClose, 'function')

// 2. Direct N-API stream cache behavior
const directCacheId = 'm40-direct-cache'
// Set memory threshold to 256 KiB
const initRaw = native.streamCacheCreate(directCacheId, 256 * 1024, null)
assert.ok(initRaw)
const initStats = JSON.parse(initRaw)
assert.equal(initStats.cacheId, directCacheId)
assert.equal(initStats.totalCachedBytes, 0)
assert.equal(initStats.isSpilledToFile, false)

// Write chunk 1: 64 KiB with pattern 0x41 ('A')
const chunk1 = Buffer.alloc(64 * 1024, 0x41)
const written1 = native.streamCacheWriteChunk(directCacheId, 0, chunk1)
assert.equal(written1, 64 * 1024)

// Write chunk 2: contiguous 64 KiB with pattern 0x42 ('B')
const chunk2 = Buffer.alloc(64 * 1024, 0x42)
const written2 = native.streamCacheWriteChunk(directCacheId, 64 * 1024, chunk2)
assert.equal(written2, 64 * 1024)

// Write chunk 3: disjoint at offset 300,000 (32 KiB) with pattern 0x43 ('C')
const chunk3 = Buffer.alloc(32 * 1024, 0x43)
const written3 = native.streamCacheWriteChunk(directCacheId, 300000, chunk3)
assert.equal(written3, 32 * 1024)

const stats1Raw = native.streamCacheStats(directCacheId)
const stats1 = JSON.parse(stats1Raw)
assert.equal(stats1.totalCachedBytes, 64 * 1024 + 64 * 1024 + 32 * 1024)
assert.equal(stats1.bufferedRangeCount, 2)
assert.equal(stats1.isSpilledToFile, false)
assert.equal(stats1.memoryBytes, stats1.totalCachedBytes)

// Read slices in RAM
const read1 = native.streamCacheRead(directCacheId, 0, 4)
assert.ok(read1)
assert.deepEqual(Array.from(read1), [0x41, 0x41, 0x41, 0x41])

const readCross = native.streamCacheRead(directCacheId, 64 * 1024 - 2, 4)
assert.ok(readCross)
assert.deepEqual(Array.from(readCross), [0x41, 0x41, 0x42, 0x42])

// Read at gap returns null
const readGap = native.streamCacheRead(directCacheId, 200000, 10)
assert.equal(readGap, null)

// Trigger disk spill: write 200 KiB (total cached becomes 160 + 200 = 360 KiB > 256 KiB)
const chunk4 = Buffer.alloc(200 * 1024, 0x44)
native.streamCacheWriteChunk(directCacheId, 128 * 1024, chunk4)

const statsSpilledRaw = native.streamCacheStats(directCacheId)
const statsSpilled = JSON.parse(statsSpilledRaw)
assert.equal(statsSpilled.isSpilledToFile, true)
assert.equal(statsSpilled.memoryBytes, 0) // RAM is freed
assert.ok(statsSpilled.fileBytes >= statsSpilled.totalCachedBytes)

// Verify reading from spilled file across boundaries
const readSpilledCross = native.streamCacheRead(directCacheId, 128 * 1024 - 2, 4)
assert.ok(readSpilledCross)
assert.deepEqual(Array.from(readSpilledCross), [0x42, 0x42, 0x44, 0x44])

// Close cache
assert.equal(native.streamCacheClose(directCacheId), true)
assert.equal(native.streamCacheStats(directCacheId), null)

// 3. Verification via bridge-runtime wrappers
const runtimeCacheId = 'm40-runtime-cache'
const rInit = nativeStreamCacheCreate(runtimeCacheId, 512 * 1024, null)
assert.ok(rInit)
assert.equal(rInit.cacheId, runtimeCacheId)

const rData = Buffer.from('STREAM_CACHE_RUNTIME_TEST_PAYLOAD')
const rWritten = nativeStreamCacheWriteChunk(runtimeCacheId, 1000, rData)
assert.equal(rWritten, rData.length)

const rRead = nativeStreamCacheRead(runtimeCacheId, 1000, rData.length)
assert.ok(rRead)
assert.equal(Buffer.from(rRead).toString(), 'STREAM_CACHE_RUNTIME_TEST_PAYLOAD')

const rStats = nativeStreamCacheStats(runtimeCacheId)
assert.ok(rStats)
assert.equal(rStats.totalCachedBytes, rData.length)

assert.equal(nativeStreamCacheClose(runtimeCacheId), true)
assert.equal(nativeStreamCacheStats(runtimeCacheId), null)

console.log('M40 Rust native stream chunk ring cache & spill storage tests passed')
