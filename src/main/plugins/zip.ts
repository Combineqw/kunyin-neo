/**
 * 只读 zip 解析（仅用 node:zlib，无第三方依赖）。
 *
 * 为什么手写：项目里唯一能解 zip 的 yauzl/extract-zip 是 electron 这个 devDependency
 * 的传递依赖，electron-builder 打包时按生产依赖裁剪，构建产物里并不存在——直接 require
 * 会「dev 能跑、装机即崩」。项目本身已有多处手写二进制解析（tag/flacBlock.ts、tag/ogg.ts），
 * 与既有风格一致。
 *
 * 支持 method 0（stored）与 method 8（deflate），足够覆盖常规 zip 打包器。
 * 不支持加密、zip64、多卷——遇到即明确报错，不做静默降级。
 */
import { inflateRawSync } from 'node:zlib'

const EOCD_SIG = 0x06054b50
const CEN_SIG = 0x02014b50
const LOC_SIG = 0x04034b50
const MAX_COMMENT = 0xffff

export interface ZipEntry {
  fileName: string
  compressionMethod: number
  compressedSize: number
  uncompressedSize: number
  localHeaderOffset: number
}

/** 从尾部反向定位 EOCD（末尾可能带注释，故需扫描）。 */
function findEocd(buf: Buffer): number {
  const min = Math.max(0, buf.length - (MAX_COMMENT + 22))
  for (let i = buf.length - 22; i >= min; i -= 1) {
    if (buf.readUInt32LE(i) === EOCD_SIG) return i
  }
  return -1
}

/** 读取中央目录，列出全部条目。 */
export function listZipEntries(buf: Buffer): ZipEntry[] {
  const eocd = findEocd(buf)
  if (eocd < 0) throw new Error('不是有效的 zip 文件（找不到中央目录）')

  const entryCount = buf.readUInt16LE(eocd + 10)
  const cenSize = buf.readUInt32LE(eocd + 12)
  const cenOffset = buf.readUInt32LE(eocd + 16)
  if (entryCount === 0xffff || cenSize === 0xffffffff || cenOffset === 0xffffffff) {
    throw new Error('不支持 zip64 格式的插件包')
  }
  if (cenOffset + cenSize > buf.length) throw new Error('zip 中央目录越界（文件可能已损坏）')

  const entries: ZipEntry[] = []
  let p = cenOffset
  for (let i = 0; i < entryCount; i += 1) {
    if (p + 46 > buf.length || buf.readUInt32LE(p) !== CEN_SIG) {
      throw new Error('zip 中央目录条目损坏')
    }
    const flags = buf.readUInt16LE(p + 8)
    if (flags & 0x1) throw new Error('不支持加密的插件包')
    const compressionMethod = buf.readUInt16LE(p + 10)
    const compressedSize = buf.readUInt32LE(p + 20)
    const uncompressedSize = buf.readUInt32LE(p + 24)
    const nameLen = buf.readUInt16LE(p + 28)
    const extraLen = buf.readUInt16LE(p + 30)
    const commentLen = buf.readUInt16LE(p + 32)
    const localHeaderOffset = buf.readUInt32LE(p + 42)
    // 名称一律按 UTF-8 解（flags bit 11 未置位的历史 zip 可能是 CP437，
    // 但插件包由我们自己的规范约定为 UTF-8）
    const fileName = buf.subarray(p + 46, p + 46 + nameLen).toString('utf8')
    entries.push({
      fileName,
      compressionMethod,
      compressedSize,
      uncompressedSize,
      localHeaderOffset
    })
    p += 46 + nameLen + extraLen + commentLen
  }
  return entries
}

/** 解出单个条目的内容。 */
export function readZipEntry(buf: Buffer, entry: ZipEntry): Buffer {
  const off = entry.localHeaderOffset
  if (off + 30 > buf.length || buf.readUInt32LE(off) !== LOC_SIG) {
    throw new Error(`zip 本地头损坏：${entry.fileName}`)
  }
  const nameLen = buf.readUInt16LE(off + 26)
  const extraLen = buf.readUInt16LE(off + 28)
  const start = off + 30 + nameLen + extraLen
  const end = start + entry.compressedSize
  if (end > buf.length) throw new Error(`zip 数据越界：${entry.fileName}`)
  const raw = buf.subarray(start, end)

  if (entry.compressionMethod === 0) return Buffer.from(raw)
  if (entry.compressionMethod === 8) {
    // zip 存的是裸 deflate 流（无 zlib 头），必须用 inflateRaw
    const out = inflateRawSync(raw)
    if (entry.uncompressedSize && out.byteLength !== entry.uncompressedSize) {
      throw new Error(`zip 条目解压后大小不符：${entry.fileName}`)
    }
    return out
  }
  throw new Error(`不支持的 zip 压缩方式（method ${entry.compressionMethod}）：${entry.fileName}`)
}

/** 按名称取条目内容（找不到返回 null）。名称匹配不区分目录分隔符写法。 */
export function readZipFile(buf: Buffer, fileName: string): Buffer | null {
  const target = fileName.replaceAll('\\', '/').toLowerCase()
  const entries = listZipEntries(buf)
  const hit =
    entries.find((e) => e.fileName.replaceAll('\\', '/').toLowerCase() === target) ??
    // 兼容多打一层顶层目录的包：xxx/manifest.json
    entries.find((e) => {
      const n = e.fileName.replaceAll('\\', '/').toLowerCase()
      const parts = n.split('/')
      return parts.length === 2 && parts[1] === target
    })
  return hit ? readZipEntry(buf, hit) : null
}
