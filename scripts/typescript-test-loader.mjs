import { access } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'

/** 打包器负责的静态资源导入（图片等）。Node 直接跑 TS 时解析不了，
 * 桩成路径字符串——校验脚本只关心色值，不关心图片内容。 */
const ASSET_RE = /\.(png|jpe?g|webp|gif|svg|avif)$/i

export async function resolve(specifier, context, nextResolve) {
  if (ASSET_RE.test(specifier)) {
    return { url: `asset-stub:${specifier}`, shortCircuit: true, format: 'module' }
  }
  try {
    return await nextResolve(specifier, context)
  } catch (error) {
    if (!specifier.startsWith('.') || !context.parentURL) throw error
    for (const suffix of ['.ts', '/index.ts']) {
      const candidate = new URL(`${specifier}${suffix}`, context.parentURL)
      try {
        await access(fileURLToPath(candidate))
        return { url: candidate.href, shortCircuit: true }
      } catch {
        // Continue with the next compatible TypeScript candidate.
      }
    }
    throw error
  }
}

export async function load(url, context, nextLoad) {
  if (url.startsWith('asset-stub:')) {
    const path = url.slice('asset-stub:'.length)
    return {
      format: 'module',
      shortCircuit: true,
      source: `export default ${JSON.stringify(path)}`
    }
  }
  return nextLoad(url, context)
}
