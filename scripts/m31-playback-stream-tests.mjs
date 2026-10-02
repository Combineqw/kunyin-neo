import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const protocol = readFileSync('src/main/audio/protocol.ts', 'utf8')
const native = readFileSync('crates/aurora-native/src/lib.rs', 'utf8')
const runtime = readFileSync('src/main/native/bridge-runtime.js', 'utf8')

assert.match(protocol, /nativeAudioStream(Create|Decrypt|Close)/)
assert.match(native, /audio_stream_(create|decrypt|close)/)
assert.match(runtime, /nativeAudioStream(Create|Decrypt|Close)/)
assert.match(protocol, /nativeStreamId/)
assert.match(protocol, /nativeAudioStreamClose\(nativeStreamId\)/)
assert.match(protocol, /rendererSignal\.addEventListener\('abort', closeNativeStream/)
assert.match(protocol, /request\.signal\)/)
assert.match(protocol, /fallbackDecryptor \?\?= fallbackFactory\(\)/)
assert.doesNotMatch(protocol, /const decryptor = spec\.ekey \? createAudioDecryptor\(spec\.ekey\) : null/)
assert.match(native, /MAX_QMC2_STREAMS: usize = 64/)
assert.match(native, /too many QMC2 stream sessions/)

console.log('M31 远程播放 QMC2 有状态会话接线检查通过')
