/**
 * 三引擎边界样本清单（固化版）
 *
 * 由来：此前边界测试是临时构造、跑完即删，无法复跑也无法回执。
 * 本文件把边界用例写成清单 + 生成器，长期可复跑。
 *
 * 用法：
 *   node scripts/boundary-fixtures.mjs           生成样本并打印清单
 *   node scripts/boundary-fixtures.mjs --clean   清理生成物
 *
 * 纪律：
 *   · 样本一律生成在 os.tmpdir()，绝不写进 test-assets（不污染真实素材）
 *   · 真实 flac 从 test-assets/songs 只读复制
 *   · 每条用例带 id / 说明 / 预期，回执按 id 对齐
 *
 * settings_io 的边界用例不在此处 —— 已固化在 shadow-modules.mjs 的
 * LOOP_CASES（10 条）。本文件补 scan 与 lyrics 两块。
 */

import { mkdtempSync, writeFileSync, copyFileSync, rmSync, readdirSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { tmpdir } from 'node:os'
import { fileURLToPath } from 'node:url'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..')
const SONGS = join(repoRoot, 'test-assets', 'songs')
const ROOT_PREFIX = 'kunyin-boundary-'

/**
 * scan 边界用例。
 * kind: real-copy 复制真 flac 改名 / text 文本假冒音频 / empty 零字节 / nonaudio 非音频
 */
export const SCAN_CASES = [
  { id: 'S1-no-separator', kind: 'real-copy', name: '没有分隔符的文件名.flac',
    desc: '文件名不含 " - " 分隔符',
    expect: 'title = 整个文件名去扩展名，artist 留空；有标签则标签覆盖' },
  { id: 'S2-uppercase-ext', kind: 'real-copy', name: '大写扩展名 - 歌手.FLAC',
    desc: '扩展名大写 .FLAC',
    expect: '识别为音频（扩展名判定不区分大小写），不得漏扫' },
  { id: 'S3-leading-separator', kind: 'real-copy', name: ' - 开头就是分隔符.flac',
    desc: '文件名以分隔符开头，artist 段为空',
    expect: 'idx > 0 不成立，整名作 title、artist 留空，不得切出空 artist' },
  { id: 'S4-multi-separator', kind: 'real-copy', name: 'A - B - C.flac',
    desc: '文件名含多个分隔符',
    expect: '按第一个分隔符切分：artist=A，title=B - C' },
  { id: 'S5-tag-read-fail', kind: 'text', name: '伪装文件 - 测试歌手.flac',
    body: 'this is not audio at all',
    desc: '文本内容但用 .flac 扩展名',
    expect: '标签读取失败不崩溃：回退文件名，duration=0，parseFailed 计 1' },
  { id: 'S6-zero-byte', kind: 'empty', name: '零字节 - 空.flac',
    desc: '零字节音频文件',
    expect: '同 S5：回退文件名，duration=0，不崩溃' },
  { id: 'S7-nonaudio-skipped', kind: 'nonaudio', name: 'readme.txt', body: 'not audio',
    desc: '非音频扩展名',
    expect: '跳过，不进曲目列表，skippedNonAudio 计 1' },
  { id: 'S8-image-skipped', kind: 'nonaudio', name: 'cover.jpg', body: 'fake image',
    desc: '图片扩展名（真实曲库常与音频同目录）',
    expect: '同 S7：跳过并计数' },
]

/** lyrics 边界用例：构造 LRC，覆盖时间轴与文本的畸形写法 */
export const LYRIC_CASES = [
  { id: 'L1-two-digit-ms', name: 'L1-两位毫秒.lrc',
    body: '[00:01.50]两位毫秒\n[00:02.75]第二行\n',
    desc: '毫秒段两位（.50）',
    expect: 'padEnd(3) 补零判为 500ms，不得读成 50ms' },
  { id: 'L2-three-digit-ms', name: 'L2-三位毫秒.lrc',
    body: '[00:01.500]三位毫秒\n[00:02.755]第二行\n',
    desc: '毫秒段三位',
    expect: '直接取三位 = 500ms / 755ms' },
  { id: 'L3-four-digit-ms', name: 'L3-四位毫秒.lrc',
    body: '[00:01.5009]四位毫秒\n[00:03.0001]第二行\n',
    desc: '毫秒段超过三位',
    expect: '截断取前三位 = 500ms / 000ms（截断非四舍五入）' },
  { id: 'L4-bom', name: 'L4-带BOM.lrc',
    body: '\uFEFF[00:01.000]带 BOM 的首行\n[00:02.000]第二行\n',
    desc: '文件以 UTF-8 BOM 开头',
    expect: 'BOM 被剥离，首行正常解析，不得因 BOM 丢首行' },
  { id: 'L5-netease-json-head', name: 'L5-网易云JSON头.lrc',
    body: '{"t":0,"c":[{"tx":"作词: "},{"tx":"某人"}]}\n{"t":0,"c":[{"tx":"作曲: "}]}\n[00:05.000]正文第一行\n[00:08.000]正文第二行\n',
    desc: '前置网易云 JSON 元数据行（真实素材有 10 个此类文件）',
    expect: 'JSON 行不匹配时间轴故丢弃，正文两行正常解析' },
  { id: 'L6-meta-tags', name: 'L6-元信息标签.lrc',
    body: '[ti:标题]\n[ar:歌手]\n[al:专辑]\n[by:制作]\n[offset:+500]\n[00:01.000]正文\n',
    desc: '标准 LRC 元信息标签',
    expect: '元信息行不作为歌词行输出，两侧须一致地丢弃或保留' },
  { id: 'L7-multi-timestamp', name: 'L7-一行多时间轴.lrc',
    body: '[00:01.000][00:31.000]副歌重复句\n[00:05.000]普通行\n',
    desc: '同一行文本挂多个时间轴（副歌复用）',
    expect: '两侧须一致：或展开为多行，或只取第一个' },
  { id: 'L8-empty-text', name: 'L8-空文本行.lrc',
    body: '[00:01.000]\n[00:02.000]有文本\n[00:03.000]   \n',
    desc: '时间轴存在但文本为空或纯空格',
    expect: '两侧一致处理（保留空行或一致丢弃）' },
  { id: 'L9-unordered', name: 'L9-时间轴乱序.lrc',
    body: '[00:10.000]第十秒\n[00:02.000]第二秒\n[00:06.000]第六秒\n',
    desc: '时间轴未按升序排列',
    expect: 'end 回填须基于排序结果，否则 end 会算成负值' },
  { id: 'L10-last-line-end', name: 'L10-末行end.lrc',
    body: '[00:01.000]第一行\n[00:05.000]最后一行\n',
    desc: '末行没有后继行可供 end 回填',
    expect: '末行 end = 0（已知瑕疵 C1，两侧一致即 PASS，修复排 R3 后）' },
  { id: 'L11-hour-timestamp', name: 'L11-超一小时.lrc',
    body: '[61:30.000]六十一分半\n[75:00.000]七十五分\n',
    desc: '分钟段超过 60（长音频/有声书）',
    expect: '分钟不取模，61:30 = 3690000ms' },
  { id: 'L12-crlf', name: 'L12-CRLF换行.lrc',
    body: '[00:01.000]CRLF 第一行\r\n[00:02.000]第二行\r\n',
    desc: 'Windows CRLF 换行',
    expect: '\\r 不得残留在文本尾部' },
  { id: 'L13-empty-file', name: 'L13-空文件.lrc',
    body: '',
    desc: '零字节 lrc',
    expect: '返回空列表，不报错' },
  { id: 'L14-no-timestamp', name: 'L14-纯文本无时间轴.lrc',
    body: '这是一段没有任何时间轴的纯文本\n第二段也没有\n',
    desc: '整个文件不含时间轴',
    expect: '返回空列表，不得把纯文本当歌词行' },
  { id: 'L15-space-in-bracket', name: 'L15-方括号内带空格.lrc',
    body: '[00:01.000] 前导空格文本\n[ 00:02.000]括号内带空格\n',
    desc: '时间轴内或文本前有多余空格',
    expect: '两侧一致：前导空格的保留/去除须相同' },
]

/** 生成 scan 样本目录 */
export function makeScanFixtures() {
  const dir = mkdtempSync(join(tmpdir(), ROOT_PREFIX + 'scan-'))
  let realSrc = null
  if (existsSync(SONGS)) {
    const flac = readdirSync(SONGS).find((f) => f.toLowerCase().endsWith('.flac'))
    if (flac) realSrc = join(SONGS, flac)
  }
  const made = []
  for (const c of SCAN_CASES) {
    const target = join(dir, c.name)
    if (c.kind === 'real-copy') {
      if (!realSrc) { made.push({ ...c, skipped: '无真 flac 来源' }); continue }
      copyFileSync(realSrc, target)
    } else if (c.kind === 'empty') {
      writeFileSync(target, '')
    } else {
      writeFileSync(target, c.body ?? '', 'utf8')
    }
    made.push({ ...c, file: target })
  }
  return { dir, made }
}

/** 生成 lyrics 样本目录 */
export function makeLyricFixtures() {
  const dir = mkdtempSync(join(tmpdir(), ROOT_PREFIX + 'lyrics-'))
  const made = []
  for (const c of LYRIC_CASES) {
    writeFileSync(join(dir, c.name), c.body, 'utf8')
    made.push({ ...c, file: join(dir, c.name) })
  }
  return { dir, made }
}

/** 清理本生成器留下的所有临时目录 */
export function cleanFixtures() {
  const base = tmpdir()
  let n = 0
  for (const name of readdirSync(base)) {
    if (!name.startsWith(ROOT_PREFIX)) continue
    try { rmSync(join(base, name), { recursive: true, force: true }); n++ } catch { /* 占用中跳过 */ }
  }
  return n
}

if (process.argv[1] && process.argv[1].endsWith('boundary-fixtures.mjs')) {
  if (process.argv.includes('--clean')) {
    console.log('已清理 ' + cleanFixtures() + ' 个样本目录')
  } else {
    const s = makeScanFixtures()
    const l = makeLyricFixtures()
    console.log('scan 样本   ' + s.made.length + ' 条 → ' + s.dir)
    console.log('lyrics 样本 ' + l.made.length + ' 条 → ' + l.dir)
    console.log('')
    for (const c of [...s.made, ...l.made]) console.log('  ' + c.id.padEnd(22) + c.desc)
  }
}