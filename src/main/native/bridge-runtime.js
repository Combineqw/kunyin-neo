/* eslint-disable @typescript-eslint/explicit-function-return-type */
import { createRequire } from 'node:module'
import { existsSync } from 'node:fs'
import { join } from 'node:path'

const require = createRequire(import.meta.url)
let binding

function nativeCandidates() {
  const file = 'aurora-native.win32-x64-msvc.node'
  const candidates = []
  if (typeof process.resourcesPath === 'string') candidates.push(join(process.resourcesPath, 'assets', file))
  candidates.push(join(process.cwd(), 'crates', 'aurora-native', file))
  if (typeof __dirname === 'string') candidates.push(join(__dirname, '..', '..', 'crates', 'aurora-native', file))
  return candidates
}

function loadBinding() {
  if (binding !== undefined) return binding
  for (const candidate of nativeCandidates()) {
    if (!existsSync(candidate)) continue
    try {
      binding = require(candidate)
      return binding
    } catch {
      // Keep the TypeScript bridge's optional-native fallback semantics.
    }
  }
  binding = null
  return null
}

export function nativeReadAudioTags(path) {
  try {
    const raw = loadBinding()?.readAudioTags(path)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}
