# 坤音 neo · kunyin-neo

<p align="center">
  <img src="./resources/icons/icon.png" width="120" alt="坤音 neo 图标">
</p>

**多平台在线音乐 + 面向发烧友的本地曲库。**

坤音 neo 基于 KunYin Desktop 持续演进，保留在线搜索、播放、歌单和歌词能力，逐步补齐本地音乐管理、音量均衡与桌面体验。当前客户端使用 Electron + Vue 3 + TypeScript；Rust 核心已由 Electron N-API 适配层接入，并维护独立的 Tauri v2 验证壳。真机主观体验仍由项目所有者验收。

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
| Rust 扫描模块 | 音频目录扫描、元数据解析，补充跳过文件、解析失败和遍历错误计数       | Electron 本地扫描优先调用 Rust，异常回退 Node |
| Rust 歌词模块 | LRC 行级解析与 Node 语义比对                                         | 共享核心已接入 N-API，生产歌词链仍保留 Node 能力 |
| Rust 设置模块 | 设置读取、序列化和沙箱回环测试，真实文件由哈希校验保护               | Electron 设置读路径优先调用 Rust，保留 Node 回退 |
| M2 扫描进度岛 | 可取消异步扫描、节流进度、批量事务写入与顶层进度展示               | 已完成，待所有者主观验收                          |
| M3 地基批 | 失焦暂停、封面取色、玻璃三档与动效基础                         | 已完成，待所有者主观验收                          |
| M4 四季极光 | 春夏秋冬主题、日期自动/手动切换与 0.28s 过渡                     | 已完成，待所有者主观验收                          |
| M5 功能批 | F1 本地信息补全、F1.5 ReplayGain、F3' 曲库管家、F2 聚合搜索       | 已完成；F4' 仅完成 WASAPI 评估                   |
| M6 双绑定 | `aurora-core` 共享 Electron N-API 与独立 Tauri v2 验证壳           | 已完成，未替换 Electron 主壳                     |
| M9 原生切片 | Tauri Rust commands：设置读写、曲库扫描、歌词解析、能力探针          | 已完成切片，Electron 主壳仍为默认产品          |
| M10-M12 Rust 数据与标签 | SQLite 曲库 CRUD、Lofty 标签读写、Electron 本地元数据桥接 | 已完成影子路径，保留兼容回退                  |
| M18-M21 Rust 播放与解码边界 | 播放会话、Electron 播放影子桥、Symphonia 本地解码 | 已完成边界，实际声音仍由 HTMLAudio 输出        |
| M22 Rust 设备输出 | Windows CPAL/WASAPI shared 默认设备、有界 PCM 队列、输出快照 | 已完成输出边界，尚未接入生产播放生命周期      |
| M23 Rust 本地播放 | Symphonia 解码、CPAL/WASAPI shared 输出、播放控制与快照；本地文件原生优先、失败回退 HTMLAudio | 已完成纵向切片；远程/加密流与 DSP 仍由现有路径负责 |
| M26 Rust 本地搜索 | SQLite FTS5 索引、中文安全回退、分页查询与 Tauri command | 已完成本地搜索切片；远程 Provider、认证和同步仍由 Electron 负责 |
| 同源测试      | 将本地音乐纯逻辑抽到 `core.ts`，生产入口与 shadow 扫描共用源码       | 已完成，消除扫描模块的复刻副本漂移              |
| 回归样本      | 固化扫描 8 条、歌词 15 条、设置回环 10 条边界用例                    | 存档记录为 33/33 PASS；用例生成与执行比对需区分 |
| 打包修复      | 修正平台级 `files` 覆盖顶层白名单，排除测试素材、Rust 工程和工具目录 | 历史 asar 从 2844.5 MB 降至 22.2 MB             |
| 工程存档      | `MEMORY.md` 记录决策与里程碑事实，随升版提交                         | 当前档案版本 v5.24；验收结论由项目所有者作出     |

2026-09-04 的 Windows 安装包记录为 **97,099,765 字节**（按 1024 换算约 **92.6 MiB**）。这是指定历史构建的记录，后续构建需重新核验，不能据此推断当前机器的性能或体验。

自动化测试和构建结果不等于整机验收通过；真机手感、卡顿、主题观感和功能清单仍由项目所有者检查。

## 规划功能

以下是后续路线与尚未承诺完成的能力。

### 功能路线

已完成 **F1 → F1.5 → F3' → F2**；F4' 目前只有评估记录，生产实现等待所有者决定：

| 顺序 | 功能                | 计划内容                                                   |
| ---- | ------------------- | ---------------------------------------------------------- |
| F1   | 本地音乐自动补全    | 已交付：按标签匹配封面、歌词和专辑名，支持可取消批量任务 |
| F1.5 | ReplayGain 音量均衡 | 已交付：读取标签并平滑应用单曲/专辑增益                   |
| F3'  | 曲库管家            | 已交付：`KUNYIN__` 清理预览/撤销、查重与健康检查           |
| F2   | 聚合搜索增强        | 已交付：六源并发、先回先显、来源标记和过期搜索作废         |
| F4'  | WASAPI 独占评估     | 先评估收益与实现成本，再决定是否实现；不作为现有能力承诺   |

### 性能、架构与体验

- **曲库基建**：增量扫描、SQLite FTS5、虚拟滚动与缓存，围绕大型曲库逐步验证。
- **在线体验**：封面缓存、歌单虚拟滚动和下一首预加载。
- **桌面体验**：继续完善扫描进度岛、封面取色、玻璃与光照、统一动效 tokens、极光返工及失焦暂停的真机验收。
- **架构演进**：Electron 继续发版，Tauri 壳保持并行验证；功能对等和测试充分后再决定发布渠道切换。

性能目标需要基准与真机数据支持；不预先承诺内存下降幅度或具体完成时间。

## 获取与运行

本仓库当前没有已发布的 GitHub Release，开发者可从源码构建；后续发布入口为 [本仓库 Releases](https://github.com/Combineqw/kunyin-neo/releases)。

当前 `electron-builder.yml` 的发布目标仍指向上游 `ikunshare/kunyin-desktop`，本分支的自动更新渠道尚未完成切换；现有客户端的更新提示不能作为本仓库发布的依据。

### 开发环境

- 推荐 **Node.js 22.18+**（22.x LTS）及 npm；影子脚本直接导入 TypeScript，使用支持默认类型擦除的 Node 版本。
- 开发客户端使用 Electron 39、Vue 3、TypeScript、electron-vite 和 electron-builder。
- 构建 Rust 核心与 N-API 模块另需 Rust/Cargo、napi-rs 构建依赖及对应平台编译工具；运行 Electron 客户端会优先使用可用的 Rust 本地模块，缺失时回退 Node。

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

## Native Rust migration

M10 extends the Tauri slice with a Rust SQLite repository (`aurora-library`)
that preserves schema v9, system playlists, ordering, redirects, and JSON song
payloads. Tauri commands now expose playlist CRUD/query operations and
read-only Lofty metadata extraction. Electron local-song parsing tries the Rust
metadata bridge first and keeps its existing fallback when the native module is
unavailable or a container cannot be read.

The Tauri shell remains a migration harness while providers, playback,
downloads, tag writing, sync, and desktop window contracts are moved in later
stages. Full Rust native migration is therefore still in progress. Verify the
slice with `cargo check --manifest-path src-tauri/Cargo.toml`,
`cargo test --manifest-path crates/aurora-library/Cargo.toml`, and
`cargo test --manifest-path src-tauri/Cargo.toml`.

The audio migration now includes a Symphonia-backed local decoder in the
shared `aurora-audio` crate. It decodes local, unencrypted files one packet at
a time into bounded interleaved `f32` PCM chunks, and exposes that capability to
the N-API adapter and Tauri shell. Desktop playback still uses the existing
Chromium HTMLAudio fallback; remote streams, provider encryption, device
output, and WASAPI exclusive mode remain later slices.

M22 adds a Windows CPAL/WASAPI shared-mode output controller. It selects the
default endpoint, exposes its sample format and bounded PCM queue, and keeps
the real-time callback non-blocking by emitting silence on queue underrun or
lock contention. This is an output boundary for the next playback slice;
Electron playback, resampling, remote/encrypted streams, and lifecycle
integration still need to move over before native output can be enabled in
production.

M23 adds the first native local-file playback slice. `NativePlaybackEngine`
combines the Symphonia decoder with a bounded PCM queue and Windows shared
output, including bounded linear resampling and deterministic channel mapping
to the default device format. It exposes play, pause, stop, seek, and snapshot
operations through six optional N-API functions. Electron tries this path for
local files and falls back immediately to its existing `kunyin://` + HTMLAudio
route when the native module cannot open the file or the native start fails.
Remote streams and encrypted provider paths are unchanged. DSP/EQ, remote
providers, downloads, encryption, and WASAPI exclusive mode remain later
slices, so the complete application is still a staged migration.

M24 moves QQ QMC2 (`mflac`/`mgg`) stream decryption into the shared Rust audio
crate. The decryptor handles ekey V1/V2 and map/RC4 range chunks by absolute
encrypted-file offset, so HTTP range requests can be decrypted independently.
Electron prefers the `audioDecryptQmc2Chunk` N-API export and keeps the
existing TypeScript decryptor as a compatibility fallback for missing native
modules, stale bindings, or invalid keys. Remote provider requests,
authentication, downloads, DSP, and desktop window integration remain staged
work; this slice does not claim complete Rust migration.

M25 adds a bounded-buffer Rust path for completed QQ encrypted downloads.
`audioDecryptQmc2File` transforms a file in place in 256 KiB chunks, preserving
the absolute offset semantics while avoiding the previous full-file read and
second full-file allocation. Electron uses it when available and falls back to
the existing TypeScript decryptor for older bindings or unsupported keys.

M26 adds a Rust SQLite FTS5 index for local-library search. Song writes and
schema migration rebuild the denormalized index from canonical JSON payloads;
ASCII queries use literalized FTS5 matching with BM25 ordering, while Chinese,
non-ASCII, and short terms use an escaped parameterized `LIKE` fallback so
substring search remains useful with SQLite's unicode61 tokenizer. The Tauri
validation shell exposes the paginated `library_search_songs` command and keeps
the existing JSON DTO shape. Remote providers, authentication, synchronization,
and the Electron production shell remain staged migration work.

M27 shares one 100 ms renderer scheduler between the mini-player and desktop
lyrics state bridges, avoiding duplicate timers while keeping their immediate
state pushes. M28 moves LX sync server URL normalization and persisted-session
validation into `aurora-core::sync`, exposed through N-API and Tauri commands;
Electron retains a TypeScript fallback. HTTP/WebSocket transport, credentials,
encryption, compression, and full sync orchestration remain on the existing
Electron path, so the full application has not yet completed Rust migration.

M29 moves QQ single-track JSON normalization into `aurora-core::provider` and
exposes it through the N-API adapter. QQ and QQC retain the existing provider
request paths while the parser uses Rust first and the TypeScript implementation
as a compatibility fallback. The provider shadow corpus passes at the JSON
boundary; remote HTTP, authentication, encryption, downloads, and the desktop
shell remain staged migration work.

M30 adds the same Rust-first JSON normalization boundary for Netease Cloud
Music single-track responses. The existing eapi/weapi, request scheduling, and
TypeScript fallback remain unchanged. Native release builds use ThinLTO and a
two-job default to keep local CPU and memory pressure bounded while the adapter
grows; `CARGO_BUILD_JOBS` can override that default for a release host.

M31 adds bounded, stateful QMC2 sessions for encrypted remote playback. Each
`kunyin://` HTTP Range response creates one Rust session, parses its ekey once,
and sends only the session id, absolute offset, and bytes for subsequent chunks.
Normal stream completion and renderer aborts close the session; missing or
failing native bindings still fall back to the existing chunk bridge and
TypeScript decryptor. HTTP, proxy, Range handling, and Chromium decoding remain
unchanged. This moves the remote buffering/decryption boundary only; complete
remote playback, provider authentication, DSP, downloads, and desktop shell
migration remain staged work.

M32 keeps the compatibility decryptor lazy: encrypted remote playback does not
construct the TypeScript QMC2 decryptor unless the Rust binding is unavailable
or a native chunk fails. This removes redundant key parsing and fallback state
from the normal Rust path while preserving the existing recovery behavior.

M33 moves theme motion profiles into `aurora-core`. Seasonal blob geometry,
timing, blur, opacity, frame interval, and reduced-motion policy are computed
by Rust and exposed through `themeMotionProfile`; the renderer only applies the
returned values as CSS variables for final compositing. This keeps animation
policy native without pretending Chromium's compositor has been replaced.

M11 also routes Electron's local enrichment, tag reads, and local-library
health probes through the Rust metadata bridge first. The existing TypeScript
tag writers and `music-metadata` fallback remain available for unsupported
containers and compatibility.

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
