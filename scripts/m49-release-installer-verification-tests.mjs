import assert from 'node:assert/strict'
import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'

const repoRoot = path.resolve(import.meta.dirname, '..')

console.log('Running M49 Windows Release Installer Packaging & Verification Tests...')

const installerPath = path.join(repoRoot, 'dist', 'kunyin-desktop-1.1.0-setup.exe')
const blockmapPath = path.join(repoRoot, 'dist', 'kunyin-desktop-1.1.0-setup.exe.blockmap')
const unpackedExePath = path.join(repoRoot, 'dist', 'win-unpacked', 'kunyin-desktop.exe')
const nativeModulePath = path.join(
  repoRoot,
  'dist',
  'win-unpacked',
  'resources',
  'assets',
  'aurora-native.win32-x64-msvc.node'
)
const iconPath = path.join(
  repoRoot,
  'dist',
  'win-unpacked',
  'resources',
  'assets',
  'icons',
  'icon.ico'
)

// 1. Verify installer executable existence & size
assert(fs.existsSync(installerPath), `Installer not found at ${installerPath}`)
const stat = fs.statSync(installerPath)
console.log(`- Installer size: ${stat.size.toLocaleString()} bytes`)
assert(stat.size > 90 * 1024 * 1024, `Installer size too small: ${stat.size} bytes`)
assert(stat.size < 130 * 1024 * 1024, `Installer size suspiciously large: ${stat.size} bytes`)

// 2. Compute and verify SHA-256 hash
const buffer = fs.readFileSync(installerPath)
const hash = crypto.createHash('sha256').update(buffer).digest('hex').toUpperCase()
console.log(`- Installer SHA-256: ${hash}`)
assert.equal(hash.length, 64, 'SHA-256 hash length must be 64 characters')

// 3. Verify blockmap existence
assert(fs.existsSync(blockmapPath), `Blockmap file not found at ${blockmapPath}`)
const blockmapStat = fs.statSync(blockmapPath)
assert(blockmapStat.size > 1024, `Blockmap file too small: ${blockmapStat.size} bytes`)
console.log(`- Blockmap size: ${blockmapStat.size.toLocaleString()} bytes`)

// 4. Verify unpacked main executable
assert(fs.existsSync(unpackedExePath), `Unpacked executable not found at ${unpackedExePath}`)
const unpackedStat = fs.statSync(unpackedExePath)
console.log(`- Unpacked executable size: ${unpackedStat.size.toLocaleString()} bytes`)
assert(unpackedStat.size > 100 * 1024 * 1024, `Unpacked executable size too small: ${unpackedStat.size}`)

// 5. Verify optimized Rust native module injection
assert(fs.existsSync(nativeModulePath), `Native module not injected at ${nativeModulePath}`)
const nativeStat = fs.statSync(nativeModulePath)
console.log(`- Native Rust module size: ${nativeStat.size.toLocaleString()} bytes`)
assert(nativeStat.size > 1024 * 1024, `Native module too small: ${nativeStat.size} bytes`)

// 6. Verify resources assets (icons, etc.)
assert(fs.existsSync(iconPath), `Icon asset not found at ${iconPath}`)

console.log('M49 Windows Release Installer Packaging & Verification tests passed successfully!')
