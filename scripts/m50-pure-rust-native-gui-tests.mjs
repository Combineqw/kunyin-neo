import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';

console.log('Running M50 Pure Rust Native GUI & Installer Verification Tests...');

const root = process.cwd();
const cargoTomlPath = path.join(root, 'crates', 'aurora-app', 'Cargo.toml');
const themePath = path.join(root, 'crates', 'aurora-app', 'src', 'theme.rs');
const statePath = path.join(root, 'crates', 'aurora-app', 'src', 'state.rs');
const libPath = path.join(root, 'crates', 'aurora-app', 'src', 'lib.rs');
const mainPath = path.join(root, 'crates', 'aurora-app', 'src', 'main.rs');
const nsiPath = path.join(root, 'crates', 'aurora-app', 'installer', 'kunyin-rust-setup.nsi');
const releaseExePath = path.join(root, 'crates', 'aurora-app', 'target', 'release', 'aurora-app.exe');
const installerPath = path.join(root, 'dist', 'kunyin-rust-1.1.0-setup.exe');

// 1. Verify Cargo.toml configuration
assert.ok(fs.existsSync(cargoTomlPath), 'crates/aurora-app/Cargo.toml must exist');
const cargoContent = fs.readFileSync(cargoTomlPath, 'utf8');
assert.ok(cargoContent.includes('aurora-core'), 'Must depend on aurora-core');
assert.ok(cargoContent.includes('aurora-player'), 'Must depend on aurora-player');
assert.ok(cargoContent.includes('aurora-library'), 'Must depend on aurora-library');
assert.ok(cargoContent.includes('aurora-audio'), 'Must depend on aurora-audio');
assert.ok(cargoContent.includes('opt-level = 3'), 'Must configure release optimization');

// 2. Verify Apple Design tokens in theme.rs
assert.ok(fs.existsSync(themePath), 'crates/aurora-app/src/theme.rs must exist');
const themeContent = fs.readFileSync(themePath, 'utf8');
assert.ok(themeContent.includes('ThemeMode'), 'Must declare ThemeMode');
assert.ok(themeContent.includes('AppleColors'), 'Must declare AppleColors');
assert.ok(themeContent.includes('AppleMetrics'), 'Must declare AppleMetrics');
assert.ok(themeContent.includes('play_button_size: 44.0'), 'Must enforce HIG 44pt touch target');
assert.ok(themeContent.includes('radius_pill: 999.0'), 'Must enforce squircle and pill radius');
assert.ok(themeContent.includes('space_lg: 16.0'), 'Must implement 4pt grid');
assert.ok(themeContent.includes('#007AFF'), 'Must include Apple primary blue');
assert.ok(themeContent.includes('#0F0F12'), 'Must include Apple dark background');

// 3. Verify reactive state & session in state.rs
assert.ok(fs.existsSync(statePath), 'crates/aurora-app/src/state.rs must exist');
const stateContent = fs.readFileSync(statePath, 'utf8');
assert.ok(stateContent.includes('PlaybackSession'), 'Must bind aurora-player PlaybackSession');
assert.ok(stateContent.includes('Library'), 'Must bind aurora-library SQLite repository');
assert.ok(stateContent.includes('AppRoute'), 'Must declare AppRoute');
assert.ok(stateContent.includes('DspSettings'), 'Must declare 10-band DSP settings');

// 4. Verify components
const components = ['sidebar.rs', 'toolbar.rs', 'player_bar.rs', 'song_table.rs', 'settings_view.rs'];
for (const comp of components) {
  const p = path.join(root, 'crates', 'aurora-app', 'src', 'components', comp);
  assert.ok(fs.existsSync(p), `Component ${comp} must exist`);
}

// 5. Verify cargo unit tests pass
const cargoBin = 'C:\\Users\\Administrator\\.cargo\\bin\\cargo.exe';
if (fs.existsSync(cargoBin)) {
  const testOutput = execSync(`"${cargoBin}" test --manifest-path crates/aurora-app/Cargo.toml`, { encoding: 'utf8' });
  assert.ok(testOutput.includes('test result: ok'), 'Cargo unit tests must pass');
  console.log('- Rust unit tests: 100% passed');
}

// 6. Verify release binary execution and diagnostics
assert.ok(fs.existsSync(releaseExePath), 'Release executable must exist');
const exeStat = fs.statSync(releaseExePath);
console.log(`- Pure Rust release executable size: ${exeStat.size.toLocaleString()} bytes (~${(exeStat.size / 1024 / 1024).toFixed(2)} MB)`);
assert.ok(exeStat.size < 10 * 1024 * 1024, 'Binary size must be under 10MB (pure native footprint)');

const versionOutput = execSync(`"${releaseExePath}" --version`, { encoding: 'utf8' });
assert.ok(versionOutput.includes('kunyin-desktop (pure rust native)'), 'Must report pure rust native binary');

const selfCheckOutput = execSync(`"${releaseExePath}" --self-check`, { encoding: 'utf8' });
const diag = JSON.parse(selfCheckOutput.trim());
assert.equal(diag.app_name, 'KUNYIN Neo (Pure Rust)');
assert.equal(diag.version, '1.1.0');
assert.ok(diag.audio_backend.includes('WASAPI'));
assert.ok(diag.gui_architecture.includes('Pure Rust'));
console.log(`- Self-check diagnostics: validated successfully`);

// 7. Verify NSIS installer script and generated release installer
assert.ok(fs.existsSync(nsiPath), 'NSIS installer script must exist');
const nsiContent = fs.readFileSync(nsiPath, 'utf8');
assert.ok(nsiContent.includes('PRODUCT_NAME "昆音 (Kunyin Neo)"'), 'NSIS script must configure product name');
assert.ok(nsiContent.includes('Uninstall.exe'), 'NSIS script must configure clean uninstaller');
assert.ok(nsiContent.includes('Kunyin.Audio'), 'NSIS script must register audio file associations');

assert.ok(fs.existsSync(installerPath), 'Generated release installer must exist');
const installerStat = fs.statSync(installerPath);
console.log(`- Windows release installer size: ${installerStat.size.toLocaleString()} bytes (~${(installerStat.size / 1024 / 1024).toFixed(2)} MB)`);
assert.ok(installerStat.size < 5 * 1024 * 1024, 'Installer size must be under 5MB');

console.log('M50 Pure Rust Native GUI & Installer Verification tests passed successfully!\n');
