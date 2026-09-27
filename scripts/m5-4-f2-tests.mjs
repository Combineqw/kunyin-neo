import assert from 'node:assert/strict'

const { runSourceSearches } = await import('../src/common/domain/aggregateSearch.ts')
const pending = new Map()
for (const source of ['slow', 'fast']) {
  pending.set(source, {})
  pending.get(source).promise = new Promise((resolve) => {
    pending.get(source).resolve = resolve
  })
}
const resolved = []
const settled = []
const search = runSourceSearches(
  ['slow', 'fast'],
  (source) => pending.get(source).promise,
  () => true,
  {
    resolved: (source, value) => resolved.push([source, value]),
    rejected: () => assert.fail('unexpected search rejection'),
    settled: (source) => settled.push(source)
  }
)
pending.get('fast').resolve(['first'])
await new Promise((resolve) => setImmediate(resolve))
assert.deepEqual(resolved, [['fast', ['first']]])
pending.get('slow').resolve(['later'])
await search
assert.deepEqual(resolved, [['fast', ['first']], ['slow', ['later']]])
assert.deepEqual(settled, ['fast', 'slow'])

let current = true
let stalePublished = false
const stale = runSourceSearches(
  ['old'],
  async () => 'stale result',
  () => current,
  {
    resolved: () => { stalePublished = true },
    rejected: () => { stalePublished = true },
    settled: () => { stalePublished = true }
  }
)
current = false
await stale
assert.equal(stalePublished, false)

console.log('M5.4 F2 聚合搜索时序检查通过')
