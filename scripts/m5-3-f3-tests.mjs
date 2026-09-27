import assert from 'node:assert/strict'

const { buildLocalCleanupPlan, cleanKunyinFilename, safeFileStem } =
  await import('../src/common/domain/localLibrary.ts')

assert.equal(safeFileStem('  A/B  '), 'A B')
assert.equal(cleanKunyinFilename('C:/music/KUNYIN__Artist - Title.mp3'), 'Artist - Title.mp3')
const plan = buildLocalCleanupPlan(
  [
    { itemKey: '1_local', filePath: 'C:/music/KUNYIN__old.mp3', title: 'Title', artist: 'Artist' },
    { itemKey: '2_local', filePath: 'C:/music/keep.mp3', title: 'Keep', artist: '' }
  ],
  new Set(['c:/music/kunyin__old.mp3', 'c:/music/keep.mp3'])
)
assert.equal(plan[0].status, 'rename')
assert.equal(plan[0].newPath, 'C:/music\\Artist - Title.mp3')
assert.equal(plan[1].status, 'unchanged')
const conflict = buildLocalCleanupPlan(
  [{ itemKey: '1_local', filePath: 'C:/music/KUNYIN__old.mp3', title: 'Keep', artist: '' }],
  new Set(['c:/music/kunyin__old.mp3', 'c:/music/Keep.mp3'])
)
assert.equal(conflict[0].status, 'conflict')
assert.equal(safeFileStem('A\u0001B'), 'A B')

console.log("M5.3 F3' 曲库管家纯逻辑检查通过")
