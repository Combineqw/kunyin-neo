import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

async function runM47Tests() {
  const rootDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  const builderYamlPath = path.join(rootDir, 'electron-builder.yml')
  assert.ok(fs.existsSync(builderYamlPath), 'electron-builder.yml must exist')

  const yamlContent = fs.readFileSync(builderYamlPath, 'utf-8')

  // 1. 验证 Windows 目标架构为 NSIS x64
  assert.ok(yamlContent.includes('target: nsis'), 'Must specify nsis target')
  assert.ok(yamlContent.includes('arch:') && yamlContent.includes('x64'), 'Must target x64 architecture')

  // 2. 验证多格式文件关联 (File Associations)
  assert.ok(yamlContent.includes('fileAssociations:'), 'Must configure fileAssociations')
  assert.ok(yamlContent.includes('ext: mp3'), 'Must associate .mp3')
  assert.ok(yamlContent.includes('ext: flac'), 'Must associate .flac')
  assert.ok(yamlContent.includes('ext: wav'), 'Must associate .wav')
  assert.ok(yamlContent.includes('ext: m4a'), 'Must associate .m4a')
  assert.ok(yamlContent.includes('ext: ogg'), 'Must associate .ogg')

  // 3. 验证 NSIS 向导式安装配置 (非单文件，带桌面与开始菜单快捷方式、许可与卸载)
  assert.ok(yamlContent.includes('oneClick: false'), 'oneClick must be false for guided wizard installation')
  assert.ok(yamlContent.includes('allowToChangeInstallationDirectory: true'), 'User must be allowed to choose installation folder')
  assert.ok(yamlContent.includes('createDesktopShortcut: always'), 'Desktop shortcut must always be created')
  assert.ok(yamlContent.includes('createStartMenuShortcut: true'), 'Start menu shortcut must be enabled')
  assert.ok(yamlContent.includes('license: build/license.txt'), 'License agreement must be configured')
  assert.ok(yamlContent.includes('zh_CN') && yamlContent.includes('en_US'), 'Must support bilingual installer')
  assert.ok(yamlContent.includes('deleteAppDataOnUninstall: false'), 'Clean uninstall protection')

  // 4. 验证资源文件真实存在
  const licensePath = path.join(rootDir, 'build/license.txt')
  assert.ok(fs.existsSync(licensePath), 'License file build/license.txt must exist')

  const iconPath = path.join(rootDir, 'resources/icons/icon.ico')
  assert.ok(fs.existsSync(iconPath), 'Installer icon resources/icons/icon.ico must exist')

  const afterPackPath = path.join(rootDir, 'scripts/afterPack.js')
  assert.ok(fs.existsSync(afterPackPath), 'afterPack.js hook must exist')
  const afterPackContent = fs.readFileSync(afterPackPath, 'utf-8')
  assert.ok(afterPackContent.includes('aurora-native.win32-x64-msvc.node'), 'afterPack.js must package native Rust addon')

  // 5. 验证编译生成的原生二进制文件存在
  const nativeBinaryPath = path.join(rootDir, 'crates/aurora-native/aurora-native.win32-x64-msvc.node')
  assert.ok(fs.existsSync(nativeBinaryPath), 'Compiled native addon must exist in crates/aurora-native/')

  console.log('M47 Windows Modern Installer Packaging & Setup Verification tests passed successfully.')
}

runM47Tests().catch((err) => {
  console.error('M47 test failed:', err)
  process.exit(1)
})
