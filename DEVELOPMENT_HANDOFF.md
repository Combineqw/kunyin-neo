# KUNYIN Neo 开发交接手册

更新时间：2026-10-10（Asia/Singapore）

## 当前基线

- 分支：`master`
- M48 工作内容：Apple Design 细节美学与质感微调（Apple Design UI Refinements & HIG Detail Polish）
- 产品版本：`1.1.0`
- 远端：`https://github.com/Combineqw/kunyin-neo.git`
- 技术栈：Electron 39、Vue 3、TypeScript、Rust、N-API-RS、纯 Rust 原生化架构规划
- 终极形态战略决策：彻底弃用 Web / WebView 运行时（不采用 Tauri，走向 100% 纯 Rust 原生 GUI 与音频引擎，发布形态采用 NSIS/MSI 现代化安装程序）

M48 在保持整体结构与桌面四区布局大体不变的前提下，按照 Apple HIG（人机界面指南）规范深度打磨 UI 质感与微交互：
1. 字体排印：body 与 page-title 补全 Apple SF Pro 字体渲染栈与微负字偶距（`-0.01em` / `-0.015em`），开启 `-moz-osx-font-smoothing: grayscale` 与 `text-rendering: optimizeLegibility`；
2. 材质与高光：玻璃基础类 `.glass-surface` 注入 Apple 标志性内边缘高光描边（`box-shadow: inset 0 0.5px 0 0 var(--glass-highlight)`），暗色模式适配内高光与边框；
3. 侧栏浮岛：`AppAside` 胶囊指示器与导航项外间距按 4pt 网格微调，增加 Apple spring 弹性曲线与微发光；
4. 顶部搜索与窗口：`AppToolbar` 胶囊搜索框注入 `backdrop-filter: blur(14px)` 毛玻璃、微边框及精致清空按钮，窗口控件调整为柔和圆角微交互；
5. 底部播放条：`PlayerBar` 播放按键升级为 44px 触感圆形大按钮，带微投影与按压弹性形变；封面增加 squircle 10px 连续圆角与微边框；音量弹层升级为 24px 毛玻璃高饱和质感；
6. 歌曲列表与卡片：`SongRow` 歌曲行内封面、品质标签升级为 Apple HIG 胶囊徽章样式；`PlaylistCard` 封面圆角升级为 14px 并注入悬停弹性浮起；`DetailHeader` 播放全部升级为高质感胶囊按钮。

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

`npm run test:m48` 覆盖 Apple Design UI 细节美学、HIG 4pt 间距、字体排印与微交互回归。全仓 lint 如果扫描到未跟踪 `work/` 脚本，应使用正式源码范围的 scoped lint，不要把临时脚本加入提交。

## 下一步建议

下一步进入 M49：Windows 安装包构建打包与发布核验（Windows Release Installer Packaging & Integrity Verification）。执行 `npm run build:win` 生成现代化 NSIS 安装程序（`kunyin-desktop-1.1.0-setup.exe`），提取产物体积与 SHA-256 校验和，确保快捷方式、开始菜单及文件关联完备。每个切片都要保留旧路径、加入确定性回归、升级 `MEMORY.md`，并单独提交和普通推送。

## 提交纪律

- 禁止 force push。
- 不提交密钥、账号、真实音乐素材、`target/`、`node_modules/`、`dist/`、`out/` 或 `work/`。
- 不修改 `PROJECT_STATUS.md`。
- 提交前执行 `git diff --check`、状态检查和敏感文件检查。
- 发布安装包使用 `npm run build:win`，记录文件大小和 SHA-256。
- 只记录事实、测试结果和已知边界，不替项目所有者下验收结论。
