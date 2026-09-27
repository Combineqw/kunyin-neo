export interface LocalCleanupInput {
  itemKey: string
  filePath: string
  title: string
  artist: string
}

export type LocalCleanupStatus = 'rename' | 'unchanged' | 'conflict' | 'missing'

export interface LocalCleanupPlanEntry {
  itemKey: string
  oldPath: string
  newPath: string
  status: LocalCleanupStatus
  reason?: string
}

function isWindowsReservedStem(stem: string): boolean {
  return /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(stem)
}

function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

function dirname(path: string): string {
  const match = path.match(/^(.*)[\\/][^\\/]*$/)
  return match?.[1] ?? ''
}

function extname(path: string): string {
  const name = basename(path)
  const dot = name.lastIndexOf('.')
  return dot > 0 ? name.slice(dot) : ''
}

function join(dir: string, name: string): string {
  if (!dir) return name
  return `${dir}${dir.endsWith('\\') || dir.endsWith('/') ? '' : '\\'}${name}`
}

function normalizeWindowsPath(path: string): string {
  return path.replaceAll('/', '\\').toLowerCase()
}

export function safeFileStem(value: string): string {
  const cleaned = value
    .replace(/[<>:"/\\|?*]/g, ' ')
    .replace(/[\p{Cc}]/gu, ' ')
    .replace(/[. ]+$/g, '')
    .replace(/\s+/g, ' ')
    .trim()
  if (!cleaned) return '未命名'
  return isWindowsReservedStem(cleaned) ? `_${cleaned}` : cleaned
}

/** Remove the downloader's KUNYIN__ marker while preserving the extension. */
export function cleanKunyinFilename(filePath: string): string {
  const name = basename(filePath)
  const extension = extname(name)
  const stem = name.slice(0, name.length - extension.length)
  const cleaned = stem.replace(/^KUNYIN__+/i, '').trim()
  return `${safeFileStem(cleaned || stem)}${extension}`
}

function preferredFilename(input: LocalCleanupInput): string {
  const current = basename(input.filePath)
  if (!/^KUNYIN__+/i.test(current)) return current
  const extension = extname(current)
  const stem = input.artist.trim() ? `${input.artist.trim()} - ${input.title.trim()}` : input.title.trim()
  return `${safeFileStem(stem)}${extension}`
}

export function buildLocalCleanupPlan(
  items: readonly LocalCleanupInput[],
  existingPaths: ReadonlySet<string>
): LocalCleanupPlanEntry[] {
  const normalizedExistingPaths = new Set([...existingPaths].map(normalizeWindowsPath))
  const planned = new Set<string>()
  return items.map((item) => {
    const oldPath = item.filePath
    const newPath = join(dirname(oldPath), preferredFilename(item))
    if (!/^KUNYIN__+/i.test(basename(oldPath))) {
      return { itemKey: item.itemKey, oldPath, newPath: oldPath, status: 'unchanged' }
    }
    const normalized = normalizeWindowsPath(newPath)
    if (normalized === normalizeWindowsPath(oldPath)) {
      return { itemKey: item.itemKey, oldPath, newPath, status: 'unchanged' }
    }
    if (
      planned.has(normalized) ||
      normalizedExistingPaths.has(normalized)
    ) {
      return {
        itemKey: item.itemKey,
        oldPath,
        newPath,
        status: 'conflict',
        reason: '目标文件已存在或与另一首歌曲冲突'
      }
    }
    planned.add(normalized)
    return { itemKey: item.itemKey, oldPath, newPath, status: 'rename' }
  })
}
