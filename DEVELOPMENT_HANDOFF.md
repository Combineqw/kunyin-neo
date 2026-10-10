# KUNYIN Neo 开发交接手册

更新时间：2026-10-10（Asia/Singapore）

## 当前基线

- 分支：`master`
- M41 工作内容：原生流式解码管线（Native Stream Decoder via Symphonia）
- 产品版本：`1.1.0`
- 远端：`https://github.com/Combineqw/kunyin-neo.git`
- 技术栈：Electron 39、Vue 3、TypeScript、Rust、N-API-RS、纯 Rust 原生化架构规划
- 终极形态战略决策：彻底弃用 Web / WebView 运行时（不采用 Tauri，走向 100% 纯 Rust 原生 GUI 与音频引擎）

M41 在 `crates/aurora-audio` 实现流式解码模块 `stream_decoder`，基于 Symphonia 驱动多格式音频解码并对接 `StreamCache`。支持自适应 Header 探测、流式包解码、欠载保护（Underflow 自动等待后续网络分块而非中断报错）、有界 `SharedPcmQueue` 缓冲泵入与精准 Seek 重置。`crates/aurora-native` 升级缓存共享架构并暴露 7 个 N-API 接口，由 `bridge.ts` 与 `bridge-runtime.js` 完成类型化导出。现有网络取流、QMC2 解密、Chromium `<audio>` 解码与 Web Audio 渲染完全保留。

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

`npm run test:m41` 需要先执行 `npm run native:build`。全仓 lint 如果扫描到未跟踪 `work/` 脚本，应使用正式源码范围的 scoped lint，不要把临时脚本加入提交。

## 下一步建议

下一块进入 M42：原生 DSP 音效引擎（Native Audio DSP Engine），实现 10 段 Biquad IIR 均衡器（Peak/Shelf，算法精度对齐 Web Audio 传函系数）、ReplayGain 原生增益应用与平滑淡入淡出（防爆音），将算法与管线彻底收拢在 Rust 音频核心中。每个切片都要保留旧路径、加入确定性回归、升级 `MEMORY.md`，并单独提交和普通推送。

## 提交纪律

- 禁止 force push。
- 不提交密钥、账号、真实音乐素材、`target/`、`node_modules/`、`dist/`、`out/` 或 `work/`。
- 不修改 `PROJECT_STATUS.md`。
- 提交前执行 `git diff --check`、状态检查和敏感文件检查。
- 发布安装包使用 `npm run build:win`，记录文件大小和 SHA-256。
- 只记录事实、测试结果和已知边界，不替项目所有者下验收结论。
