# kunyin-neo 客观采集记录

日期：2026-09-06；时间均为 UTC+08:00。本文为工单5及同轮配对补充指令终稿，只记录操作和观测，不作验收通过判定。

## 仓库与授权链

- 仓库：https://github.com/Combineqw/kunyin-neo
- 可见性：Public；前次权限核查时，唯一具备仓库写入权限的账号为所有者 Combineqw，无待接受的协作者邀请。公开可读不授予访客直接推送权限；MIT 仍允许修改各自的副本。
- 原两笔提交 b06947f、73d1151 已推送。首次 CDP 补采提交前远端 HEAD 为 `fdabcc24699847a5ca1968502929a4fb5f5bd01f`，包含 README 重写；首次报告与记忆提交 `df57e07a11a3cab41c849e4419105e4b1b974ac1` 已推送，同轮配对记录在其后追加。
- README 提交标题为“docs(readme): 重写项目介绍、功能规划与作者署名”，仅修改 README.md，新增 85 行、删除 119 行，涵盖上游来源、已修改功能、规划功能、运行与构建、权限、许可及署名。
- 作者署名：Codex、清言、Claude、南风知我意 (@Combineqw)；上游 ikunshare 与 MIT 版权声明保留。
- 公开设置及 README 由 Codex 按用户后续原话执行：“公开仓库但是不允许其他人修改，readme你自己写一份新的，包含修改功能，项目来自，规划功能，作者写你，清言，Claude，我”。
- 人类署名依据用户补充：“按我github id来，南风知我意”。该后续公开授权覆盖工单3的 Private 要求。
- 本单只提交本报告及 MEMORY.md 的 #16、#17、#18 追加记录；未 force push，未修改 release.yml、package.json、PROJECT_STATUS.md 或锁定功能。
- 用户确认“+57 −1”没有另一个版本，是上一轮界面的待审核编辑，现按用户指令作废；以当前可见草稿为定稿基础。

## 安装包与显示模式

- 用户确认采用实际路径：`D:\kunyin-neo\dist\kunyin-desktop-1.0.7-setup.exe`；原工单文件名多出的“1.0-”已由用户确认采用同 SHA 的实际文件。
- 大小：97,099,765 字节。
- SHA-256：`d0da5315861423a4f07b69c0c0aecc2342f497dd3db67d223e59c67bf4b44055`，与指定基线一致，安装前再次核验。
- 安装方式：NSIS `/S`；退出码 `0`；已安装产品版本 `1.0.7.0`。
- 安装位置：`C:\Users\Administrator\AppData\Local\Programs\kunyin-desktop`。
- 系统查询：`Get-CimInstance Win32_VideoController`。
- 显卡：NVIDIA GeForce RTX 2070 SUPER；显示模式：2560 × 1440，**165 Hz**。
- 165 Hz 是系统报告的当前显示模式刷新率，不是应用实测帧率，也不是显示器支持的最高刷新率。

## 内存读数

| 组别 | 采样时间 | 失焦观察时长 | 应用进程数 | Working Set 合计 | Private Bytes 合计 |
| --- | --- | --- | --- | --- | --- |
| 正式包 A，未暂停版 | 13:08:44 | 30.468 秒 | 5 | 608.00 MiB | 290.15 MiB |
| dev A，未暂停版 | 13:12:51 | 30.510 秒 | 4 | 449.70 MiB | 223.14 MiB |
| dev B（CDP 自动，跨轮记录） | 13:54:41 | 30.534 秒 | 4 | 426.58 MiB | 209.00 MiB |
| 同轮配对 A（CDP 自动） | 14:08:07 | 30.536 秒 | 4 | **426.46 MiB** | **203.87 MiB** |
| 同轮配对 B（CDP 自动） | 14:08:38 | 30.510 秒 | 4 | **426.93 MiB** | **204.02 MiB** |

原始字节数：正式包 A 为 Working Set 637,534,208 / Private Bytes 304,242,688；dev A 为 471,539,712 / 233,979,904；dev B 为 447,299,584 / 219,152,384。

同轮配对原始字节数：A 为 Working Set 447,180,800 / Private Bytes 213,770,240；B 为 447,664,128 / 213,934,080。

### 共同口径

- 使用与 A 相同的 PowerShell 采集脚本。按完整路径筛选 `Get-Process`：正式包为安装目录的 kunyin-desktop.exe，dev 为 `D:\kunyin-neo\node_modules\electron\dist\electron.exe`。
- 主进程及同路径的所有 Electron 子进程均计入，包含桌面歌词相关进程；不包含 npm、Vite、CDP 客户端、终端或其他应用进程。B 的四个进程核对为同一个主进程及其三个子进程。
- 主口径为 `WorkingSet64` 求和，辅助口径为 `PrivateMemorySize64` 求和；1 MiB = 1,048,576 字节。
- Working Set 合计可能重复计算共享页面，不能等同于系统实际释放的物理内存；Private Bytes 也不等于任务管理器的专用工作集。
- 数值为失焦至少 30 秒后逐进程读取的一次快照，不是 30 秒均值，也不是所有进程同时读取的原子快照。
- 约每秒检查前台进程及主窗口状态，要求应用失焦、主窗口可见且未最小化；检查为离散采样，无法排除间隔内的短暂状态变化。“可见”是窗口状态，不要求完全无遮挡。
- 主测主题为人鱼姬，页面为“设置 / 外观与界面”。正式包 A 保留原有暂停曲目；dev 使用同主题的空闲页面、未在播放。CDP 确认 B 页面为 `#/settings`，`data-theme=aurora_mermaid`。
- 正式包快捷键受限，故其 A 保留为独立用户基线，B 按工单5使用 dev。13:12 的历史 dev A 与 13:54 的跨轮 dev B 属于不同启动轮次，后者另启调试端口；14:08 的新增 A/B 属于同一启动轮次。按用户要求仅列数字，不计算差值，也不跨正式包/dev 相减。
- 不同启动轮次、运行时长、缓存、GC、调试服务及历史 DevTools 使用均可能影响读数；本记录不用于证明暂停动画的内存收益。

### 跨轮 B 的 CDP 操作与证据

1. 临时启动命令为 `npm run dev -- --remoteDebuggingPort 9222`。electron-vite 将该参数转为 Electron 的 `--remote-debugging-port=9222`，已通过主进程命令行核验，未改仓库启动配置。
2. `curl.exe --fail --silent --show-error http://127.0.0.1:9222/json/list` 返回播放器和桌面歌词两个 page target；选择播放器 `http://localhost:5173/#/settings`，target ID 为 `D325E13934BE0E3682973EB266B67A0D`。
3. 使用现有 Node v24.19.0 的内置 WebSocket，发送 `Runtime.evaluate` 执行 `document.getAnimations().forEach(a => a.pause())`。一次性脚本位于系统临时目录，不入仓、不提交。
4. 13:52:59.057 执行成功；暂停前动画列表为 `[]`，暂停后为 `[]`，暂停数量 **0**。因此 B 表示“暂停命令执行后的内存快照”，不能证明当时实际暂停了任何动画。
5. 13:52:59.280 WebSocket 已断开，客户端退出。全程未打开 GUI DevTools；调试端口随应用在采样期间保持开启，其影响未单独扣除。
6. 用户回复“已失焦”后，13:54:10.978 开始观察，13:54:41.510 读取进程；31 次状态观察均通过，主窗口未最小化，应用未获得焦点。
7. 标注：**dev B（CDP 自动）；CSS 暂停近似，rAF 未覆盖；本次动画枚举数量为 0**。命令范围仅为播放器主文档，不覆盖桌面歌词文档，也不代表失焦暂停功能已实现。
8. CDP 首试成功，未启用人工备路。采样后已关闭临时 dev，确认仓库路径下的 Electron 进程退出，9222 与 5173 端口不再监听；已安装正式包保留。

| B 进程 ID | 进程角色 | Working Set 字节 | Private Bytes 字节 |
| --- | --- | --- | --- |
| 8048 | network utility | 56,397,824 | 14,680,064 |
| 16860 | main | 156,987,392 | 97,841,152 |
| 17256 | renderer | 110,690,304 | 49,676,288 |
| 18640 | renderer | 123,224,064 | 56,954,880 |
| 合计 | 4 个进程 | 447,299,584 | 219,152,384 |

### 同轮配对（CDP 自动）

在上述跨轮 B 之后，按用户补充指令重新启动一次 dev + 9222，同一运行实例内按下表顺序采集；全程用户零操作，未打开 GUI DevTools。

| 时点 | 时间 | getAnimations().length | running / paused 数量 | document.hasFocus() | Working Set / Private Bytes |
| --- | --- | --- | --- | --- | --- |
| ① 聚焦态 | 14:06:57.285 | 0 | 0 / 0 | true | 不采内存 |
| ② 失焦 30 秒后 A | 内存 14:08:07.615；DOM 14:08:07.727 | 0 | 0 / 0 | false | 426.46 / 203.87 MiB |
| ③ CDP 暂停后 | 14:08:07.730 | 0 | 0 / 0 | false | 不采内存 |
| ④ 暂停命令后再失焦 30 秒 B | 内存 14:08:38.769；DOM 14:08:38.826 | 0（补充复核） | 0 / 0 | false | 426.93 / 204.02 MiB |

- 聚焦由 CDP `Page.bringToFront` 完成。14:06:57.140 的系统观察确认前台 HWND=1967160、PID=19180，均为播放器主窗口；随后 DOM 确认 hasFocus=true。再通过窗口工具自动激活资源管理器，使播放器真实失焦，未最小化。
- A 从 14:07:37.079 开始计时，观察 30.536 秒；B 在暂停命令后，从 14:08:08.259 重新计时，观察 30.510 秒。两段各 31 次离散状态观察均为失焦、可见、未最小化，前台进程 PID 均为 9904（资源管理器）。
- 同一主进程 PID=19180，启动时间 14:03:07.762；主窗口 HWND=1967160，CDP target=`C9E74795CCF624D10D15EC6AA599E1FD`，performance.timeOrigin=1788674598071.9，A/B 进程集合均为 10812、19180、19460、23232。
- 四次 DOM 记录均为 `http://localhost:5173/#/settings`、人鱼姬 `aurora_mermaid`、视口 1020×660、`--app-zoom: 1.0625`、空闲未播放；visibilityState 均为 visible。上述页面字段与实例标识前后核对一致。
- 暂停命令仍为 `document.getAnimations().forEach(a => a.pause())`。四次动画列表均为 `[]`；length、running 和 paused 均为 0。length 是枚举数量，不等于运行中动画数；本轮未观察到实际被暂停的动画。
- A/B 均调用此前未修改的 `work/measure-kunyin-memory.ps1`，使用同一 PowerShell 7.6.5 宿主。两段等待及内存读取时 WebSocket 均已断开，调试服务均保持开启；A 之后重连读取 length 并执行暂停，14:08:07.732 再次断开，之后才开始 B。
- DOM 读数紧随内存快照、非同一原子采样；A 的 DOM 时间晚约 0.112 秒，B 晚约 0.057 秒，分别保留时间。样本为一次顺序配对，仅列客观数字，不作性能或验收结论。
- 标注仍为“CSS 暂停近似，rAF 未覆盖”，仅作用于播放器主文档。采集后关闭 dev，确认 Electron 进程退出，9222 与 5173 无监听。临时 CDP/窗口观察脚本、work 下原测量脚本及 JSON 均未入仓。

此前 GUI 路线仅列出播放器与桌面歌词，根因猜想是自动快捷键未送达目标渲染页或 DevTools 窗口未被枚举；未经验证，本单不深挖。

## 原始记录与用户自跑项

原始 JSON 与自测清单保存在执行工作区的 outputs 目录，不随本次两文件提交上传；本报告已内嵌关键数值与口径。

| 本地文件 | SHA-256 |
| --- | --- |
| system-and-installer.json | `1869c97813831255fc2b171efafa6943a03d53cf53396b79639a139ccf77a4f3` |
| memory-packaged-A.json | `2ce42eb0f2e3d221e2a604579178ed18b1f3ce20f692a8276b55fe6b027ca168` |
| memory-dev-A.json | `cd6d51ef1fe8910c2ae97180ebdedd2ec79dbf4e8de2419bfd0b110e888b18c0` |
| memory-dev-B.json | `653bb7b410d21bfe4df3e37a5c5b5552c58f04f96132cab075bf544c73cce641` |
| cdp-dev-B-pause.json | `1a1f6a8696b8697ca442a5b0a0d92cc9402a5f23db7d2d7ce6719c0c4120e43f` |
| paired-focus.json | `bf21e4bf9283a3124d85507cf601009f6b15fcf8998593f94cf7febd2b5f94d7` |
| memory-paired-A.json | `b0a71d3b11a1f8f6de4e456e1bcdac3da08e417676c5e5a13d674a493a3c2fdb` |
| paired-after-A-and-pause.json | `430666bf25bcac2964a8756171c863c588fa3223e7b7ae9eba8d09baa193844a` |
| memory-paired-B.json | `70a63d026d25e497e7d1e43bc1220866331d364c850ae0754a11646ee3a9c196` |
| paired-summary.json | `a12d5b18ac143bba266cebed3370675e3116bc6ee91bcbb0da7771daaf2ee8aa` |

用户主观项一页清单为 kunyin-neo-user-checklist.md，仍由用户自跑：

- 20 分钟卡顿日记，补标失焦时段并单列。
- 玻璃拿起手感。
- Network 封面查重，区分请求与实际下载。
- 极夜、晨雾、霞光、深海各 30 秒冒烟，不混入人鱼姬主测数字。
- 功能清单的声明与实际表现对照。

主观项未由执行侧代测；空白不代表正常或通过。R2-4、扫描进度岛、地基批仍锁定，最终判定权在用户。
