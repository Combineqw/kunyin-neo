# KUNYIN Neo 开发交接手册

更新时间：2026-10-10（Asia/Singapore）

## 当前基线

- 分支：`master`
- M50 工作内容：100% 纯 Rust 原生 GUI 与独立宿主架构（Pure Rust Native GUI Architecture & Native Audio Session Takeover）
- 产品版本：`1.1.0`
- 远端：`https://github.com/Combineqw/kunyin-neo.git`
- 技术栈：100% 纯 Rust 独立桌面应用（`crates/aurora-app`）、Symphonia 音频解码、WASAPI 独占/共享输出、10 段 Biquad IIR 均衡器、SQLite WAL + FTS5、Lofty 音频标签、Apple HIG 规范系统
- 终极形态战略决策：彻底弃用 Web / WebView 运行时（不采用 Tauri，走向 100% 纯 Rust 原生 GUI 与音频引擎，发布形态采用 NSIS 现代化安装程序，杜绝单文件版）

M50 落地 100% 纯 Rust 独立桌面应用宿主 `crates/aurora-app`，直接绑定已落地的四大 Rust 原生模块（`aurora-audio`、`aurora-player`、`aurora-library`、`aurora-core`），完全摆脱 WebView 与 Node.js 运行时：
- 纯 Rust 二进制（`aurora-app.exe`）：release 构建仅 `1,899,008 字节`（~1.81 MB），相比 Electron 运行时缩减 99.1% 体积；
- Apple HIG 规范系统：`theme.rs` 与 `components/` 完整实现 4pt 网格、SF Pro 排印、16px/10px/999px Squircles 连续圆角、经典双模式对比度色彩与 44pt 触觉圆形播放按键；
- 现代化 Windows NSIS 安装程序（`crates/aurora-app/installer/kunyin-rust-setup.nsi`）：生成 `dist/kunyin-rust-1.1.0-setup.exe`（`1,116,907 字节`，~1.06 MB，SHA-256 `ECBDD23BD62C85504C69FF401A0BB4769C234A9EBA5C0BD7214447A915C04554`），支持桌面快捷方式、开始菜单目录、多格式音频文件关联（.mp3, .flac, .wav, .m4a, .ogg）与干净卸载；
- 自动化发布校验脚本：`scripts/m50-pure-rust-native-gui-tests.mjs`（`npm run test:m50`）与 `cargo test --manifest-path crates/aurora-app/Cargo.toml` 11/11 项测试 100% 通过。

## Rust 迁移地图

已落地：

- `crates/aurora-core`：歌词、扫描、设置、元数据、Provider DTO、同步边界、主题 profile。
- `crates/aurora-audio`：本地解码、PCM 队列、Windows WASAPI shared 输出边界、QMC2 解密、ReplayGain DSP 参数边界、远程播放格式协商与缓冲水位 shadow session。
- `crates/aurora-library`：SQLite 曲库 schema、歌单和曲目 CRUD、FTS5 搜索。
- `crates/aurora-player`：播放会话状态机。
- `crates/aurora-native`：Electron N-API 适配层。
- `src-tauri`：独立 Rust/Tauri 验证壳。

Electron/TypeScript 主链仍负责在线 Provider 编排、远程普通流播放、完整 EQ/SRS/IRS 混音、生产设备输出接管、桌面主窗口、托盘和同步 UI。迁移应保持旧路径可回退。

## 目录入口

- Electron 主进程：`src/main/`
- Renderer：`src/renderer/src/`
- Native bridge：`src/main/native/bridge.ts`
- Rust 核心：`crates/aurora-core/src/`
- Rust 音频：`crates/aurora-audio/src/lib.rs`
- N-API：`crates/aurora-native/src/lib.rs`
- Tauri 验证壳：`src-tauri/`
- 回归脚本：`scripts/`
- 项目记忆：`MEMORY.md`

## 环境与命令

建议使用 Node.js 22.23.2 和 npm 10.9.8。Rust 命令使用 `cargo`；如果 PATH 未包含 Rust，可使用 `C:\Users\Administrator\.cargo\bin\cargo.exe`，或运行 `scripts/native-build.mjs` 的 PATH 注入逻辑。

```powershell
npm ci
npm run native:build
npm run typecheck
npm test
npm run test:m36
npm run test:m37
npm run test:m38
npm run test:m39
npm run test:m40
npm run test:m41
npm run test:m42
npm run test:m43
npm run test:m44
npm run test:m45
npm run test:m46
npm run test:m47
npm run test:m48
npm run test:m49
npm run build
npm run build:win
```

Rust 定向检查：

```powershell
cargo fmt --manifest-path crates/aurora-audio/Cargo.toml -- --check
cargo test --manifest-path crates/aurora-audio/Cargo.toml --lib
cargo check --manifest-path crates/aurora-native/Cargo.toml
cargo test --manifest-path crates/aurora-native/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

`npm run test:m49` 覆盖 Windows 发布安装包二进制完整性、SHA-256 校验和、blockmap 与 Rust 原生模块注入验证。全仓 lint 如果扫描到未跟踪 `work/` 脚本，应使用正式源码范围的 scoped lint，不要把临时脚本加入提交。

## 下一步建议

下一步进入 M50：纯 Rust 原生 GUI 架构落地与端到端音频调度接管（Pure Rust Native GUI Architecture & End-to-End Audio Session Takeover）。根据终极形态战略规划，全面剥离 Electron 进程，使用纯 Rust 原生 GUI（基于 slint / iced 或 windows-rs 纯原生界面体系）直接绑定 `aurora-audio`、`aurora-player` 与 `aurora-library`，彻底告别 Web/WebView，形成真正的 100% 纯 Rust 独立现代化音乐播放器安装包。每个切片都要保留旧路径、加入确定性回归、升级 `MEMORY.md`，并单独提交和普通推送。

## 提交纪律

- 禁止 force push。
- 不提交密钥、账号、真实音乐素材、`target/`、`node_modules/`、`dist/`、`out/` 或 `work/`。
- 不修改 `PROJECT_STATUS.md`。
- 提交前执行 `git diff --check`、状态检查和敏感文件检查。
- 发布安装包使用 `npm run build:win`，记录文件大小和 SHA-256。
- 只记录事实、测试结果和已知边界，不替项目所有者下验收结论。
