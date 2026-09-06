# 坤音 neo · kunyin-neo

<p align="center">
  <img src="./resources/icons/icon.png" width="120" alt="坤音 neo 图标">
</p>

**多平台在线音乐 + 面向发烧友的本地曲库。**

坤音 neo 基于 KunYin Desktop 持续演进，保留在线搜索、播放、歌单和歌词能力，逐步补齐本地音乐管理、音量均衡与桌面体验。当前客户端使用 Electron + Vue 3 + TypeScript，Rust 模块处于影子比对阶段，真机体验验收仍待完成。

- 项目仓库：[Combineqw/kunyin-neo](https://github.com/Combineqw/kunyin-neo)
- 项目所有者：[南风知我意 (@Combineqw)](https://github.com/Combineqw)
- 协议：[MIT License](./LICENSE)

## 项目来自哪里

本项目衍生自 [ikunshare/kunyin-desktop](https://github.com/ikunshare/kunyin-desktop)，感谢原作者 **ikunshare** 及上游贡献者提供基础播放器与功能实现。

上游提供了 Electron 桌面框架、多音源搜索播放、歌词、歌单、本地导入和主题等基础能力。本仓库以 `eef16b5` 为本地改动对照基线，独立维护后续修改，并保留原项目的 MIT 许可证和版权声明。

## 作者与协作

**Codex · 清言 · Claude · [南风知我意 (@Combineqw)](https://github.com/Combineqw)**

南风知我意是项目所有者与维护者，负责方向、需求和最终验收；Codex、清言、Claude 为 AI 协作署名。上述署名用于本分支的维护与协作，上游原作归属见“项目来自哪里”及许可证。

## 现有基础功能

以下能力继承自上游，在本分支持续维护：

- **在线音乐**：聚合网易云音乐、QQ 音乐、QQ 音乐云、酷狗、酷我和 JOOX 等来源，提供歌曲、专辑与歌手搜索。
- **播放与歌单**：多档音质播放和下载、收藏、最近播放、自建歌单、在线歌单与专辑浏览。
- **歌词**：逐行、逐字、翻译与音译歌词，以及桌面歌词悬浮窗。
- **本地音乐**：导入本地音频、解析标签，读取失败时回退文件名。
- **桌面设置**：多主题、字体与窗口设置、代理设置，以及歌单同步和导入导出。

在线功能的可用性取决于对应来源、账号权限与网络情况。

## 本分支已做的修改

| 方向          | 已完成内容                                                           | 当前状态                                        |
| ------------- | -------------------------------------------------------------------- | ----------------------------------------------- |
| 桌面交互      | 极光分隔条、点击弹性回弹、歌词点击跳转                               | 实现已提交，真机体验仍待验收                    |
| Rust 扫描模块 | 音频目录扫描、元数据解析，补充跳过文件、解析失败和遍历错误计数       | 影子比对阶段，尚未替换生产实现                  |
| Rust 歌词模块 | LRC 行级解析与 Node 语义比对                                         | 影子比对阶段                                    |
| Rust 设置模块 | 设置读取、序列化和沙箱回环测试，真实文件由哈希校验保护               | 影子比对阶段                                    |
| 同源测试      | 将本地音乐纯逻辑抽到 `core.ts`，生产入口与 shadow 扫描共用源码       | 已完成，消除扫描模块的复刻副本漂移              |
| 回归样本      | 固化扫描 8 条、歌词 15 条、设置回环 10 条边界用例                    | 存档记录为 33/33 PASS；用例生成与执行比对需区分 |
| 打包修复      | 修正平台级 `files` 覆盖顶层白名单，排除测试素材、Rust 工程和工具目录 | 历史 asar 从 2844.5 MB 降至 22.2 MB             |
| 工程存档      | 提交项目记忆 `MEMORY.md`，保留演进路线与决策依据                     | v4.4 已入库；版本变更由项目所有者发起           |

2026-09-04 的 Windows 安装包记录为 **97,099,765 字节**（按 1024 换算约 **92.6 MiB**）。这是指定历史构建的记录，后续构建需重新核验，不能据此推断当前机器的性能或体验。

Rust 模块的测试 PASS 也不等于整机验收通过。三引擎接入（R2-4）、扫描进度岛与动效地基批仍待真机验收数据齐全并由所有者确认后开展。

## 规划功能

以下均为规划，完成后才会移入已实现列表。

### 功能路线

按 **F1 → F1.5 → F3' → F2 → F4'** 的顺序推进：

| 顺序 | 功能                | 计划内容                                                   |
| ---- | ------------------- | ---------------------------------------------------------- |
| F1   | 本地音乐自动补全    | 按标签补封面、歌词和专辑名，缓存歌词，支持可取消的批量任务 |
| F1.5 | ReplayGain 音量均衡 | 分析并保存曲目增益，减少切歌时的响度差异                   |
| F3'  | 曲库管家            | 清理 `KUNYIN__` 文件名，提供预览与撤销；补充查重和损坏检测 |
| F2   | 聚合搜索增强        | 在线来源并发搜索、先回先显、来源标记和过期搜索作废         |
| F4'  | WASAPI 独占评估     | 先评估收益与实现成本，再决定是否实现；不作为现有能力承诺   |

### 性能、架构与体验

- **曲库基建**：增量扫描、SQLite FTS5、虚拟滚动与缓存，围绕大型曲库逐步验证。
- **在线体验**：封面缓存、歌单虚拟滚动和下一首预加载。
- **桌面体验**：扫描进度岛、封面取色、玻璃与光照、统一动效 tokens、极光返工及失焦暂停。
- **架构演进**：先在 Electron 中逐步接入 Rust 核心，随后维护并行 Tauri 壳，经功能对等和测试后再决定发布渠道切换。

性能目标需要基准与真机数据支持；不预先承诺内存下降幅度或具体完成时间。

## 获取与运行

本仓库目前尚未发布 Release 安装包，可从源码运行。后续发布入口为 [本仓库 Releases](https://github.com/Combineqw/kunyin-neo/releases)。

当前 `electron-builder.yml` 的发布目标仍指向上游 `ikunshare/kunyin-desktop`，本分支的自动更新渠道尚未完成切换；现有客户端的更新提示不能作为本仓库发布的依据。

### 开发环境

- 推荐 **Node.js 22.18+**（22.x LTS）及 npm；影子脚本直接导入 TypeScript，使用支持默认类型擦除的 Node 版本。
- 开发客户端使用 Electron 39、Vue 3、TypeScript、electron-vite 和 electron-builder。
- 构建 Rust 影子模块另需 Rust/Cargo、napi-rs 构建依赖及对应平台编译工具；仅运行当前 Electron 客户端无需接入 Rust 引擎。

```bash
git clone https://github.com/Combineqw/kunyin-neo.git
cd kunyin-neo
npm install
npm run dev
```

### 检查与构建

```bash
# ESLint、类型检查、核心域、主题对比度和动效约束
npm run verify

# 编译应用
npm run build

# Windows 安装包
npm run build:win
```

Windows 也可使用根目录的 `build-kunyin.cmd` / `build-kunyin.ps1`。仓库附带的 `tools/node/` 是 Windows x64 构建工具，不随应用打包。构建产物位于 `dist/`。

macOS / Linux 构建入口分别为 `npm run build:mac` 和 `npm run build:linux`；本分支的跨平台体验需分别验证。

`npm run native:compare` 用于 Node/Rust 影子比对，需先准备本机对应的原生模块产物及测试素材。扫描和歌词的边界样本可通过 `node scripts/boundary-fixtures.mjs` 生成；该命令只生成样本，不代表比对测试已经执行或通过。

## 数据目录

| 系统    | 默认目录                                                        |
| ------- | --------------------------------------------------------------- |
| Windows | `%APPDATA%/kunyin-desktop`                                      |
| macOS   | `~/Library/Application Support/kunyin-desktop`                  |
| Linux   | `$XDG_CONFIG_HOME/kunyin-desktop` 或 `~/.config/kunyin-desktop` |

## 仓库权限与反馈

仓库公开可读，**写入权限由项目所有者控制**，普通访客无法直接推送或修改本仓库。欢迎通过 [Issues](https://github.com/Combineqw/kunyin-neo/issues) 反馈问题；Pull Request 只是变更提案，是否合并由所有者决定。

公开代码仍按 MIT 许可证提供，允许他人在遵守许可证的前提下克隆、fork 和修改自己的副本。这与本仓库的写入权限是两回事。

## 许可与致谢

本项目沿用 [MIT License](./LICENSE)，保留 **Copyright (c) 2026 ikunshare** 及上游版权声明。感谢 KunYin Desktop、Electron、Vue、Rust 及所用开源依赖的贡献者。

请尊重音乐与封面的版权，使用具有合法访问权限的内容。
