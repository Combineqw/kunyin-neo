# KUNYIN Neo 开发交接手册

更新时间：2026-10-10（Asia/Singapore）

## 当前基线

- 分支：`master`
- M42 工作内容：原生 DSP 音效引擎（Native Audio DSP Engine）
- 产品版本：`1.1.0`
- 远端：`https://github.com/Combineqw/kunyin-neo.git`
- 技术栈：Electron 39、Vue 3、TypeScript、Rust、N-API-RS、纯 Rust 原生化架构规划
- 终极形态战略决策：彻底弃用 Web / WebView 运行时（不采用 Tauri，走向 100% 纯 Rust 原生 GUI 与音频引擎，发布形态采用 NSIS/MSI 现代化安装程序）

M42 在 `crates/aurora-audio` 实现原生 DSP 音效流水线 `dsp` 模块。包含 10 段 Direct Form II Transposed 双二阶 IIR 均衡器（Cookbook 传函标准，$\pm 0.01\text{ dB}$ 自动 Bypass）、ReplayGain 增益级、20ms 帧步进平滑音量渐变（Smooth Gain Ramp，杜绝瞬态爆音）、软饱和限幅器（Soft Limiter，在振幅超过 0.95 时平滑三次饱和压缩，杜绝硬削顶失真）与实时峰值统计。`crates/aurora-native` 暴露 6 个 N-API 接口，由 `bridge.ts` 与 `bridge-runtime.js` 导出强类型绑定。现有网络取流、Chromium `<audio>` 与 Web Audio 混音 100% 保持不变。

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

`npm run test:m42` 需要先执行 `npm run native:build`。全仓 lint 如果扫描到未跟踪 `work/` 脚本，应使用正式源码范围的 scoped lint，不要把临时脚本加入提交。

## 下一步建议

下一步进入 M43：原生音频渲染管线与 WASAPI 端点直通（Native Audio Sink & WASAPI Pipeline Direct Pump），将 StreamDecoder ➔ NativeDspPipeline ➔ SharedPcmQueue ➔ NativeOutputController（WASAPI Shared/Exclusive）无缝串联为端到端全链路原生音频流，完成脱离 Chromium `<audio>` 与 Web Audio 的闭环直通。每个切片都要保留旧路径、加入确定性回归、升级 `MEMORY.md`，并单独提交和普通推送。

## 提交纪律

- 禁止 force push。
- 不提交密钥、账号、真实音乐素材、`target/`、`node_modules/`、`dist/`、`out/` 或 `work/`。
- 不修改 `PROJECT_STATUS.md`。
- 提交前执行 `git diff --check`、状态检查和敏感文件检查。
- 发布安装包使用 `npm run build:win`，记录文件大小和 SHA-256。
- 只记录事实、测试结果和已知边界，不替项目所有者下验收结论。
