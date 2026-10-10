# KUNYIN Neo 开发交接手册

更新时间：2026-10-10（Asia/Singapore）

## 当前基线

- 分支：`master`
- M49 工作内容：Windows 安装包构建打包与发布核验（Windows Release Installer Packaging & Integrity Verification）
- 产品版本：`1.1.0`
- 远端：`https://github.com/Combineqw/kunyin-neo.git`
- 技术栈：Electron 39、Vue 3、TypeScript、Rust、N-API-RS、纯 Rust 原生化架构规划
- 终极形态战略决策：彻底弃用 Web / WebView 运行时（不采用 Tauri，走向 100% 纯 Rust 原生 GUI 与音频引擎，发布形态采用 NSIS/MSI 现代化安装程序）

M49 完成 Windows 现代化安装程序构建打包与产物完整性核验。执行 `npm run build:win`，生成现代化 NSIS 向导式安装包（`dist/kunyin-desktop-1.1.0-setup.exe`）：
- 安装器大小：`99,602,204 字节`（~95.0 MB），SHA-256 `4C8117C7F839C3FDF9F783A679A7C2D9DE2354D0FD91D7F4D3B9668A46A5271D`；
- 解包主程序（`dist/win-unpacked/kunyin-desktop.exe`）：`211,232,768 字节`，SHA-256 `B35DE52365815A2C6F6B0A55BB09BC806F05E11C924A6B76F488AD2E95BA7716`；
- Rust 原生模块（`dist/win-unpacked/resources/assets/aurora-native.win32-x64-msvc.node`）：`5,122,560 字节`，SHA-256 `0761FC1EC20AE5E0D07563FD9467FC6E38717317DC6AC8E314E02C769D83F6C2`；
- 自动化发布校验脚本：`scripts/m49-release-installer-verification-tests.mjs`（`npm run test:m49`）自动化断言安装器大小、SHA-256 校验和、blockmap、主程序与原生优化模块的完整性。

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
