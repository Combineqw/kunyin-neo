【kunyin-neo 项目记忆 v5.5 — 2026-09-27】
（整合 v1~v3.1 补丁 + v4.0~v4.3，本版为唯一权威版本，
  旧版本全部作废；自本版起记忆落盘仓库，随升版同步提交。
  v4.4 交接修订：执行侧 dsh → codex，2026-09-05；
  v4.7 M0 收尾记档：2026-09-26；v4.8 M1 接入记档：2026-09-27；
  v4.9 M2 进度岛记档：2026-09-27；v5.0 M3 地基批记档：2026-09-27；
  v5.1 M4 四季极光主题记档：2026-09-27；v5.2 M5 F1 本地信息补全记档：2026-09-27；
  v5.3 M5 F1.5 ReplayGain 记档：2026-09-27；v5.4 M5 F3' 曲库管家记档：2026-09-27；
  v5.5 M5 F2 聚合搜索交互与六源编排记档：2026-09-27）

◆ 项目身份
- kunyin-neo，fork 自 github.com/ikunshare/kunyin-desktop
  （MIT）
- 上游 = 六源在线聚合流媒体播放器（网易云/QQ/酷狗/酷/
  JOOX 等，Electron+Vue3，README 自述纯 AI 生成）
- 差异化主线 = 上游全网聚合 + 发烧级本地曲库，二合一
- 角色：我 = 用户/拍板人；执行侧 = codex（2026-09-05
  接手，交接书即其工单）；dsh = 前执行侧（R1~R2-3、
  10 笔提交、边界样本、侦察数据为其成果，照常引用）

◆ 当前进度（关键路径）
- R1 前端：✅ 收口
- R2-1/2/3 三引擎 Rust 化：✅ 正式存档（2026-09-04，
  边界样本 33/33 PASS：scan 8 + lyrics 15 + settings 10，
  settings 整块 1584 项 0 不一致）；样本固化于
  boundary-fixtures.mjs（os.tmpdir 生成不污染
  test-assets，可复跑可回归，此后动引擎必须过此网）；
  样本依据 4373e97（182 行）已入库
- 打包修复（第 10 笔 0ec463e）：平台级 files 覆盖顶层
  白名单致 asar 涨至 2.8GB（=C5），已修——asar 2844.5
  → 22.2MB；包体基线 92.6MB（97099765 B），SHA-256
  d0da5315861423a4f07b69c0c0aecc2342f497dd3db67d223e59
  c67bf4b44055，构建 2026-09-04 22:38:12；白名单纪律 =
  test-assets / crates / tools 永久不入产物
- 静态极光基线就绪：5 主题（人鱼姬/极夜/晨雾/霞光/
  深海），.aurora-band 通用光带，ext 覆写
  --aurora-c1/c3/glow，--anim-dur-aurora: 7s；主测人鱼
  姬（:root 默认值），余四题 30s 冒烟不进数字
- 真机验收：暂缓（用户设备送修）——改期不取消，协议见
  下节，数据何时有何时补
- R2-4（三引擎接入）：锁。解锁条件 = ①真机验收数据齐
  且过。拆步解封例外仅限：MEMORY.md 落盘、影子模块
  纯逻辑抽取（交接单在途）
- 侦察成果（2026-09-04 由 dsh 完成，移交 codex 使用）：
  local-music/index.ts 75 行——纯逻辑 = AUDIO_EXTENSIONS
  (L15) / localSongId(L18-21, SHA-1 前 13 hex) /
  parseFileName(L24-31, " - " 首个分隔符) /
  parseLocalSong(L34-64, 动态 import music-metadata)；
  唯一 electron 依赖 = pickLocalSongs(L67-75,
  dialog.showOpenDialog + getMainWindow)；唯一引用方 =
  src/main/ipc/handlers/library.ts(L14)。约束：@common
  为构建期别名（tsconfig.node.json 与 electron.vite
  .config.ts 各一处），裸 node 解析不了——抽出模块类型
  须走 import type + 相对路径（运行时擦除不影响解析）
- 上游同步策略（定档）：基线 eef16b5，自有 25 文件 /
  改动上游 31 文件；26 个为 ≤6 行对称增删低冲突；同步
  热点 = base.css（+105）、PlayerView.vue（+77），每次
  同步重点 diff 此两件；同步保持常规频率

◆ 验收协议（v4.3 定，v4.4 补正）
- 主题：人鱼姬主测，余四题各 30s 冒烟
- 卡顿日记 20min：前台使用为主；中途切离的时段回来补标
  「失焦时段」——后台重绘账归 C4，不污染前台基线；
  Network 面板顺手查封面重复下载
- 失焦 30s 内存 = 双读数：
  A = 原样失焦（动画照跑 = 真实起点，标注「未暂停版」），
    不用于对账；任务管理器读应用组合计（Electron 多进程）
  B = DevTools console 一行 document.getAnimations()
    .forEach(a => a.pause()) 后立刻关掉 DevTools 再失焦
    30s（DevTools 自身吃内存，不关会虚高污染 A−B）；
    标注「CSS 暂停近似，rAF 未覆盖」，对账主口径
  A−B = 失焦暂停在本机的价值量化（C4 论据）；换壳对账
    规则：Tauri（实现暂停后）对 B 读数，不对 A
  正式包若 DevTools 禁用：A/B 均改 npm run dev 测（标
  「dev 口径」保可比），packaged 的 A 照记作真实用户基线
- 查实际刷新率 + 功能清单 + 玻璃「拿起」体验照旧
- 归账靶点（已侦察）：backdrop-filter 7 处 /
  setInterval 6 处 / requestAnimationFrame 10 处

◆ 长期规划母题
- 三轴同归于"人肉纪律 → 系统自治"：性能 = 卡顿日记 →
  自动基准进 CI；动效 = 四问红线 → stylelint 机器执法；
  功能 = 一次性工具 → 骑在基建上的服务
- 终局：资产复利期——越用越快越懂你，新功能边际成本趋零
- 三时期：建设期（眼下~换壳）/ 体验期（换壳~T3）/
  运营期（T3 后）；建设期每件事都是地基，没有一件白干
- 上游关系：合作不依附；transport 层 + 模块隔离保证
  "停跟上游、独立发展"永远是可行选项

◆ 功能轴（已封单，五件无候补）
排序：F1 → F1.5 → F3' → F2 → F4'
- F1 自动补全⭐：本地歌联网补封面/歌词/专辑名，认标签
  不认文件名；歌词本地缓存离线；500 首批量补全可取消
- F1.5 ReplayGain 音量均衡⭐（从 F4 拆出提前）：分析半蹭
  F1 解码趟（同趟算 gain 存库，边际成本≈0），应用半现有
  音频栈乘系数即成。诚实账：gain 无损，重采样留 T3，
  完整发烧验收 T3 补
- F3' 曲库管家（瘦身版）：洗 KUNYIN__ 脏文件名 = 主菜
  （可预览可撤销）；查重/损坏检测 = 次要 tab
- F2 聚合搜索增强：交互半（先回先显/标来源/连续输入作废
  前次）前端冻结后即可做；网络半（六源并发编排）随 T2。
  纯在线聚合，不掺本地
- F4' WASAPI 独占（评估制）：先评估再决定，结论可能是
  "不做"；位深/采样率直通、无级切换同属 T3 音频管线
- 载体：扫描进度岛 = 功能轴基建（F1/F3 骑上；长任务右上
  浮岛显示进度可取消，不弹模态不阻塞）
- 功能轴 = 搬运路线：每做一件 F，永久搬走一块 Rust 领土
  （F1=取色+下载器+封面缓存；F3'=指纹+解码探测）

◆ 性能轴
- 四本账：A 壳 / B 本地曲库 / C 在线流媒体 / D 视觉
  A：Tauri 渐进迁移；包基线 92.6MB（2026-09-04 实测）→
     目标 15MB（唯一可亮倍数项）
  B：增量扫描（指纹+mtime，重开 <2s）+ SQLite FTS5
     （万首搜索 <100ms）+ 虚拟滚动；万首预案五千首触发
  C：六源并行搜索（首批 <500ms）+ 封面三级缓存 + 在线
     歌单虚拟滚动 + 预加载下一首（命中切歌 <1s 无破音）
  D：动画红线（只动 transform/opacity）+ 玻璃面积纪律
     + 三态逃生门 + 取色缓存（命中零计算）
- 八模块三梯队：
  一（随迁移）：失焦内存降级（WebView2 set_memory_
    usage_level）/ IPC 批量纪律 / 启动三波序列（1s 见脸）
  二（R2-4 后，F 轨基建）：增量扫描 / FTS5 / moka 缓存
  三（T3）：WASAPI 独占 + 位深采样率直通 + 无级切换
- 性能预算表：冷启动可交互 <1.5s / 万首增量开库 <2s /
  万首搜索 <100ms / 滚动 p95 ≤16.7ms / 失焦 30s 内存
  回落可见 / 预加载命中切歌 <1s 无破音 / 500 首批量补全
  中界面满帧
- 腐烂防护三件：criterion 基准进 CI；启动/内存每版定时
  测画趋势线；性能债登记制（当场入账，不许口头债）
- 诚实账：Windows 上 Tauri 仍用 WebView2，渲染进程内存
  与 Electron 相当，省的是主进程；断崖式内存下降不承诺，
  安装包数字是真的
- 卡顿日记纪律：每笔卡顿归入四本账之一，归不进 = 漏网
  当场补账；三嫌疑犯：极光 blur 重绘 / 玻璃大面积 /
  封面无缓存

◆ 渐进转 Rust 路线
- 架构：crate 纯核心 + 双绑定（现阶段 napi-rs 挂 Electron
  主进程；换壳后加 Tauri command 绑定，core 一行不改）；
  前端 transport 层统一 backend.call()，换壳日只切底层
- 并行壳策略：main = Electron 持续发版，tauri 分支长期养
  新壳，功能对等 → beta → 切发布渠道 → 旧壳维护期 →
  清算；切换 = 改渠道，可回滚
- 七阶段：0 三引擎(done) → 1 R2-4 接入 → 2 长跑搬运期 →
  3 网络层 Rust 化（直译不优化，上游活跃区故最后）→
  4 换壳日（已饿瘦成切渠道）→ 5 T3 音频 → 6 清算遗产
- 搬运判定三问：纯计算还是胶水？搬走删多少 Node 代码？
  上游活跃文件否？
- 搬运顺序：FTS5/取色/指纹 → 封面缓存/下载器 → 网络层
- 诚实账：双绑定维护成本；网络层分叉 = 主动分叉点；
  音频无中间态（T3 仍原子跳）；构建链复杂化

◆ 动效轴
- 策略：密而不炫；参照 Apple HIG 四原则
- 三批节奏：地基批（R2-4 期）→ 效果批（换壳后）→ 运营期
- 优先五件：封面飞行⭐ / 封面呼吸⭐ / 大标题折叠 / 氛围
  呼吸 / 指示条滑动
- 候补池：页面方向推入 / 红心爆发 / 拖拽手感 / hover 跟手
  / 数字滚动 / 搜索瀑布 / 失焦变暗 / 空状态微动 / 频谱压底
- tokens 自治：M1 从文档变代码；新控件只选语义不写动画；
  stylelint 机器执法——禁动画 width/height/top/left/
  box-shadow/filter；PR 附 tokens 清单
- Tangibles 桌面质感：虚拟光源 135deg + data-lighting
  总闸，押下必须变暗；双影烘焙；玻璃三档 thin/regular/
  thick（20/40/60px + sat 1.4/1.6/1.8）+ 层级自动绑定 +
  高光描边；圆角 28/20/12 递减；拖拽彗尾影 250ms；拿起
  四状态 IDLE/HOVER(1.02)/ACTIVE(0.96 变暗)/RELEASE
  (160ms 过冲)；常驻相互打光 = 坑不做（替代：光带+接触
  高光+彗尾影）；设置组「桌面质感」= 玻璃+光照+动效
  三态；降级链 prefers-reduced-transparency → 实色
- 澎湃4 抄单：一镜到底⭐（全局硬条款：所有"返回"回原位，
  验收 10 入口 10 回原位）/ 拖放落点光⭐（事件驱动）/
  扫描进度岛⭐（已转正功能轴）/ 双材质（通透/柔和，柔和
  挂夜间联动）/ 弹性动效（160ms 弹簧收编 tokens）
- 极光（返工）：目标流动 ARGB；3~4 大色块 transform 漂移
  8~15s 错开 / 禁动 background/blur / 低分辨率烘焙 /
  失焦暂停（=C4 同批落地）/ 静态版保留 = 降级版 / 颜色
  全收 CSS 变量（色彩总线地基）；联动三件压返工后
- 取色：Material Color Utilities TS 版；管线 64×64 降采样
  → quantize+score → HCT 色阶 → 语义角色；铁律：大面积
  低色度（16-32）、小面积高色度（32-48）；黑白灰/低色度
  封面：饱和度强制归零仅保留 L 值，色相回退系统极光蓝
  #4F8CF7 兜底。策略锁定，改动需过评审。执行读法：同一
  张低色度封面双产出——中性角色灰阶保明暗气质，彩色角色
  回退 #4F8CF7 保可点击信号；非整体变灰亦非整体变蓝；
  WCAG AA 自动调档；切歌 OKLCH 插值 400ms 交叉溶解
  （色相走短弧）；缓存按曲目 ID/封面键；与 F1 共用解码
- 最小损伤三件套：合成器独占 / 烘焙模糊 / 暂停纪律
  （失焦 paused；will-change 只给持续动画层）；三态总
  开关只管氛围类；减弱模式弹簧换透明度（替换不删除）
- 点击手感：全局 .pressable——按下 60ms ease-out scale
  0.96；松开 160ms cubic-bezier(0.34,1.56,0.64,1) 过冲；
  只动 transform；验收 = 快速连点 10 次即时跟手；红心
  爆发 = pointerup 粒子 ≤8 个 600ms 自删

◆ 已拍板决策（不可翻案除非用户主动）
1. Tauri 渐进迁移定案（crate 双绑定 + 并行壳七阶段）
2. 性能纪律：绝对值优先，倍数只进括号；唯一例外安装包；
   100ms 以下不做
3. 动效参照 HIG 四原则；新动效四问准入
4. F2 = 纯在线聚合，不掺本地；F5 系统整合全砍
5. fork 纪律：改动收敛独立模块，少碰上游文件，保 MIT；
   上游同步全强度验收
6. 功能轴性价比原则；功能轴 = 搬运路线
7. 基线纪律：基线 = 如实记录起点，先优化后测 = 对照作废；
   测量协议属测量技术不算实现代码
8. 存档介质纪律（v4.4 新立）：项目记忆落盘仓库 MEMORY.md，
   随每次升版同步提交；聊天记录仅为沟通载体，仓库为
   唯一权威存档
9. 空数据不裁决：验收简报等条件句必须数据填齐才生效，
   空数据先回问（2026-09-05 流程纠正案）
10. 执行侧交接（2026-09-05）：dsh → codex，起因 = 用户
    设备故障；交接物 = v4.4 记忆 + 仓库；dsh 成果全数
    保留引用，回切需对账后由用户拍板

◆ 缺陷/技术债登记（登记制：不留口头债）
- C1 末行 end=0：LRC 末行无结束时间；挂 R3 后批
- C3 end 回填未排序（优先级 > C1）：乱序/一行多时间轴时
  end<start 倒挂，该行永久不显示；复刻自上游既存缺陷，
  按判据 PASS 成立。修法 = 按 start 稳定排序后回填，
  双引擎同批改 + 对答案，挂 R3 后
- C4 失焦暂停未实现：visibilitychange / animation-play-
  state / blur 监听全 0 处，极光失焦仍跑 7s 循环。修法 =
  并入地基批极光返工同批；挂 R2-4 后地基批
- C5 平台级 files 覆盖顶层白名单（已修复，随第 10 笔）：
  平台级 files 替换而非合并顶层 files；基线期无大文件被
  掩盖，R2 引入素材后 asar 2.8GB 触发 NSIS 32 位 makensis
  2GB mmap 上限。修法 = 三平台段各重列白名单再追加排除
  （已修）；防复发 = YAML 注释 + 本条目 + 素材不入产物
  纪律；备选（记档不执行）：asar 再涨可换 64 位 NSIS
- 影子模块契约漂移（修复在途）：shadow 复刻生产逻辑，
  上游改生产代码则 PASS 失真。修法 = 纯逻辑抽成不依赖
  electron 的模块，生产与 shadow import 同源（交接单在
  途，完工即销账）

◆ 已砍清单（防复活）
波形进度条 / 唱片黑胶模式 / F5 系统整合全家 / F2 掺本地
搜索 / 常驻控件相互打光 / 堆叠通知及其一切改道 / F6 数据
底座全家（将来备份方案单独再提）/ 频谱（压候补池底）

◆ 上游已有（不重做）
逐字/翻译/音译歌词、桌面歌词悬浮窗、多主题、本地导入
（基础）、骨架屏、静态极光主题

◆ 眼下待办
1. codex 两笔（交接单在途）：①MEMORY.md 落盘（本全文）
   ②影子模块纯逻辑抽取（判据见交接书）
2. 真机验收：暂缓待设备，协议照旧；数据到 → 出 v4.5
  （验收记录节）
3. 仍锁：三引擎接入 → 扫描进度岛 → 地基批（极光返工含
   C4 / 取色 / 色彩总线 / 玻璃基础 / M1 tokens 成文）
   ——等验收过

◆ 工单5 客观采集与授权链追加（2026-09-06）

#16 客观数
- SHA 核对一致：d0da5315861423a4f07b69c0c0aecc2342f497dd3db67d223e59c67bf4b44055。
  实际安装包 kunyin-desktop-1.0.7-setup.exe，97099765 B，
  用户已确认采用该文件；NSIS /S 退出码 0，安装版本 1.0.7.0。
- 系统查询当前显示模式 2560×1440、165 Hz（RTX 2070 SUPER）；
  此为系统刷新率，不是应用帧率。
- 历史内存 A 双口径（依次 Working Set / Private Bytes）：
  正式包 608.00 / 290.15 MiB，dev 449.70 / 223.14 MiB。
  均为失焦至少 30 秒后 Get-Process 全应用进程求和的一次快照，
  非 30 秒均值；Working Set 可重复计共享页，Private Bytes
  非任务管理器专用工作集。1 MiB = 1048576 B。

#17 B 数值与采集方式
- dev B（CDP 自动）：2026-09-06 13:54:41（UTC+08），失焦
  30.534 秒，4 进程；Working Set 426.58 MiB（447299584 B），
  Private Bytes 209.00 MiB（219152384 B）。沿用 A 的采集脚本。
- 临时 npm run dev -- --remoteDebuggingPort 9222，由脚手架
  传为 Electron --remote-debugging-port=9222；Node v24.19.0
  内置 WebSocket 经 Runtime.evaluate 执行
  document.getAnimations().forEach(a => a.pause())。
  全程免 GUI DevTools；临时脚本不入仓，采样前断开 WebSocket。
- 人鱼姬设置页；暂停前后动画列表均为空，实际暂停数量 0。
  标注“CSS 暂停近似，rAF 未覆盖”；只证明命令执行及本轮内存
  读数，不证明动画暂停收益。B 与历史 A 非同一启动轮次，且带
  调试端口，不计算 A−B。用户确认失焦，窗口保持打开未最小化。
- CDP 首试成功；临时 dev 已关闭，调试与渲染服务端口已释放。
  终稿与详细证据见 kunyin-neo-acceptance-report.md。

同轮配对补记（CDP 自动，2026-09-06，UTC+08）：
- 按用户补充指令重启 dev + 9222，同一运行实例自动聚焦与失焦，
  用户零操作。①14:06:57.285 聚焦态 getAnimations().length=0，
  document.hasFocus()=true，系统前台为播放器主窗口。
- ②失焦 30.536 秒后 A（14:08:07.615）：4 进程，Working Set
  426.46 MiB（447180800 B）/ Private Bytes 203.87 MiB
  （213770240 B）；14:08:07.727 读取 length=0。
- ③14:08:07.730，CDP 执行暂停命令后 length=0。
- ④暂停命令后重新失焦计时 30.510 秒，B（14:08:38.769）：
  同一组 4 进程，Working Set 426.93 MiB（447664128 B）/
  Private Bytes 204.02 MiB（213934080 B）；14:08:38.826
  补充复核 length=0。四次动画列表均空，running/paused 均为 0。
- 主进程 PID=19180、主窗口 HWND=1967160、target ID 及页面
  timeOrigin 保持一致；人鱼姬设置页、视口及缩放、未播放状态
  未变。A/B 各 31 次状态观察均为失焦、窗口可见且未最小化。
- 沿用同一 ps1 和 PowerShell 7.6.5；两段等待及内存读取时
  WebSocket 均断开，调试端口均开启。DOM 观察较内存快照分别
  晚约 0.112 / 0.057 秒，非原子采样。与此前跨轮 B 并列保留，
  只记录数字，不计算差值或作验收结论；rAF 未覆盖。
- 测后 dev 已关闭，端口已释放；临时脚本及 work 下测量脚本
  不入仓、不提交，原始 JSON 留在执行工作区 outputs。

#18 授权链记录
- 仓库公开和 README 重写均由 codex 按用户原话授权执行：
  “公开仓库但是不允许其他人修改，readme你自己写一份新的，包含修改功能，项目来自，规划功能，作者写你，清言，Claude，我”。
  随后用户补充：“按我github id来，南风知我意”。
- 后续 Public 授权覆盖工单3 Private 要求；普通访客无直接推送
  权限，MIT 仍允许修改自己的副本。人类作者为南风知我意
  (@Combineqw)，与 Codex、清言、Claude 并列协作署名。
- README 提交 fdabcc24699847a5ca1968502929a4fb5f5bd01f，
  仅 README.md，+85/-119；该提交已推送，为首次补采提交前 origin
  顶端，包含前两笔 b06947f / 73d1151；本单新提交另行回执。
- 工单5授权本次 CDP 补采、报告终稿及 #16/#17/#18 追加提交。
  以上仅客观数与授权记账，不构成验收通过。用户主观项待自跑；
  R2-4、扫描进度岛、地基批仍锁，判定权仍在用户。
- 首次 CDP 采集终稿已于 df57e07a11a3cab41c849e4419105e4b1b974ac1
  提交推送；后续用户授权在其基础上追加上述同轮配对四项，提交
  范围仍仅报告与 MEMORY.md。用户确认“+57 −1”没有另一版本，
  现作废处理，按当前可见草稿定稿。

追加行（2026-09-12，采集侧原始数据）：
- 封面查重：同曲切回重复请求 = 未采到（页面停在设置页，未发生歌曲点击，
  无 URL 证据）；跨曲共用 = 未采到（同上）。原始监听日志留在
  `work/cover-cdp-events.jsonl`，脚本留在 `work/cover-cdp-listener.mjs`，
  均未跟踪、未提交。
- getAnimations：播放+动效可见时 length = 未采到；设置页空闲采样
  `document.getAnimations().length = 0`，不作为播放动效样本。
- git 核验：4b6458 内 MEMORY.md 实际版本 = v4.4；工作区 = 改动（未跟踪
  `PROJECT_STATUS.md` 与本次 `work/` 采集目录，且本地有未推送提交）。

追加行（2026-09-12，采集侧原始数据，本轮 CDP）：
- 封面查重：共记录 27 次 `Image` 请求，19 个唯一 URL。同曲切回重复请求 = 有；`https://p4.music.126.net/3mi073axgjg-g-79ObwwEQ==/109951171836582062.jpg` 在「最佳损友 / 陈奕迅」下于 14:54:55.187/.199/.201 以及切走「富士山下」后回切于 14:55:14.434/.437/.441 各请求 3 次；跨曲共用 = 本轮无法确认，「葡萄成熟时」和「富士山下」时段未捕获可归属请求，初始搜索批量的 19 次请求均标注为「未在播放」。
- getAnimations：播放操作期间采到 `#/player` 页面 14:54:56 的 `length = 13`；同轮每秒采样见到 0、1、2、3、4、12、13 等值，仅记数据不作判定。
- git 核验：`git log --oneline -5` 顶端为 `1c79e35` （下一为 `e4c147e`）；`git show 4b6458:MEMORY.md | head -n 5` 显示实际版本为 v4.4，与工单预期 v4.6 不同；工作区为改动（`MEMORY.md` 已修改，`PROJECT_STATUS.md` 与 `work/` 未跟踪）。
- 进程处置：本轮采集后已关闭 dev 与 CDP 监听器，9222/5173 端口均已释放；原始日志留在 `work/cover-cdp-events.jsonl`。

◆ M0 收尾闭环记档（2026-09-26，事实与回执）
- 版本疑点钉死：`git show 4b6458:MEMORY.md | head -n 5` 的档头实际为
  v4.4；本文件在本次记档升为 v4.7。历史审查材料中的 v4.5/v4.6
  不是 4b6458 中的实际档头。
- 本地收拢链：`git log 4b6458..1c79e35` 包含 `e4c147e`（按 URL 后缀
  修正音频 MIME，并补歌词页顶部拖动区）与 `1c79e35`（window:moveBy
  IPC、preload 暴露及歌词页 Pointer Events 拖动，交互控件排除拖动）。
  本次未使用 force push。
- 新构建：`npm run build:win` 退出码 0；安装器
  `kunyin-desktop-1.0.7-setup.exe` = 97,105,491 B，SHA-256
  `5961AB2CE94153094DD1480A070BA68CD852F1A9447AE913A89E6918ECA253AA`；
  `dist/win-unpacked/kunyin-desktop.exe` 版本 1.0.7.0，211,232,768 B，
  SHA-256 `CB0B6D2524840F4DA97ED7DB81206A5DED4BA499B1026E832856EF08B6DB90F1`。
- 安装复核：SHA 在安装前核对为上述值；NSIS `/S` 加独立安装目录探针
  退出码 0，安装目录主程序存在，版本 1.0.7.0，主程序 SHA 与解包产物一致。
  早先默认目录探针曾返回退出码 2，未作为成功依据；清理后独立目录
  复核结果为本条记录的有效结果。
- 镜像与工程检查：`npm ping --registry=https://registry.npmmirror.com`
  返回 `PONG`；镜像源 `npm install --ignore-scripts` 与 `npm ci
  --ignore-scripts` 均完成。正式源码范围的 lint（排除未跟踪 `work/`）、
  typecheck、core/theme/animation 三组测试均通过；未跟踪 CDP 脚本未纳入
  正式代码，也未提交。
- 影子回归：当前 `test-assets` 口径为 scan 58 首、lyrics 4677 行、
  settings_io 1551 项，三表均 0 mismatch；历史存档素材为 scan 51 首、
  settings 1584 项，另有早期 2454 首记录。两组数据集与计数口径分开保留，
  不混报；Rust 相对 Node 的当前 shadow 运行结果未见性能倒退。
- 客观采集沿用既有账：系统查询 165 Hz；正式包 A 608.00/290.15 MiB、
  dev A 449.70/223.14 MiB；跨轮 dev B 426.58/209.00 MiB；同轮配对
  A 426.46/203.87 MiB、B 426.93/204.02 MiB。封面监听账为 27 次
  Image 请求、19 个唯一 URL，同曲切回出现重复 URL；跨曲共用本轮无法确认；
  播放页采到 `document.getAnimations().length = 13`，只记事实不作判定。
- R2-4 解封依据：用户 2026-09-26 原话授权“全计划跑完 + 新增四季极光主题
  + 远期 Rust 原生化，全权委托”。后续里程碑仍按 M1 → M7 顺序推进，
  每段独立提交、回归、推送和升版；验收结论留给用户。

◆ M1 三引擎接入（2026-09-27，事实与回执）
- 设置读路径：`src/main/store/settings.ts` 优先调用 Rust `readSettings`，
  native 模块不可用时回退 Node `readFileSync`；既有迁移、合并、钳制与原子写保持不变。
- 本地曲库扫描：新增 Rust 优先的目录扫描 IPC、preload 契约和歌单右键入口；
  Rust 结果映射为 `LocalMusicItem`，跳过非音频/解析失败/遍历错误计入返回统计；
  native 不可用时递归回退既有 Node `music-metadata` 解析。扫描仍是同步 Rust 调用，
  长任务进度与取消留待 M2。
- 歌词边界：生产 `PLAYER_LYRIC` 继续由 Node 负责边车编码、缓存、Provider 和增强
  LRC；Rust `scanLyrics` 仅保留影子对照，未直接替换生产路径，避免丢失翻译、逐字
  时序和 GB18030 行为。
- 影子回归：scan 58/58、lyrics 4677/4677、settings_io 1551/1551，均 0 mismatch；
  当前运行均 PASS，未触碰真实文件。
- 工程检查：`npm run typecheck`、定向 lint、`npm test` 均通过；`npm run native:compare`
  通过。未跟踪 `work/` 脚本不纳入正式 lint。
- Windows 构建：`npm run build:win` 退出码 0；安装器 97,562,099 B，SHA-256
  `26ECB812E3F94F27D6C9A2F725B0C255A19DFCDDD60577CE4344F4B52616932F`；
  解包主程序 1.0.7.0，SHA-256 `482CCFE9C6E406106540884C34841619C285EDBA56A4DC98866AB9821559538B`；
  `resources/assets/aurora-native.win32-x64-msvc.node` 1,459,712 B，SHA-256
  `A279140364970F086EABD567AE9940396F91DDBA8C8CDBB16FB52E845C69C3F5`。
- 原生二进制按既有 `.gitignore` 保持未跟踪；干净检出构建前需执行
  `npm run native:build`，本次未提交本机 `.node`、`target/`、`work/`。
- 可复现打包：移除 `electron-builder.yml` 对被忽略 `.node` 的静态引用，改由
  `scripts/afterPack.js` 在 Windows x64 本机构建产物存在时复制到
  `resources/assets`；干净 CI 无该文件时记录警告并使用 Node 回退，不阻断打包。

◆ M2 扫描进度岛（2026-09-27，事实与回执）
- 扫描入口改为可取消的异步任务：目录收集与逐文件元数据解析之间让出事件循环，
  进度事件按约 80ms 节流；任务取消后不进入数据库写入阶段。
- IPC 新增扫描进度事件和取消通道；同一时间只允许一个活动扫描任务，避免多个任务
  竞争同一进度岛。进度岛提升到 `App.vue`，播放器顶层路由切换时仍保持订阅。
- 扫描完成后使用 `addToPlaylistBatch` 单事务写入，曲库只广播一次；重复项、非音频、
  解析失败和遍历错误计入 skipped。M2 用户扫描路径逐文件调用 Rust 解析，原生模块
  缺失时回退 Node；Rust 整目录同步接口继续保留用于 M1 影子/兼容路径。
- 原生回归：新增 `parseTrack` 后重新构建成功；scan 58/58、lyrics 4677/4677、
  settings_io 1551/1551，均 0 mismatch；单文件 Rust 解析实测返回标题与时长字段。
- Windows 构建与安装：`electron-builder --win` 退出码 0（输出定向到 C 盘临时目录；
  D 盘空间不足时首次 `npm run build:win` 失败，错误为 `ENOSPC`）；安装器
  97,727,425 B，SHA-256
  `C13A9B4920DF7DA018232C603E72D25E5982868B41629F50B85F647D7DE6998F`；
  解包主程序版本 1.0.7，SHA-256
  `CC6A2D7F566DB57DDEDB9FB3BE484F49C1833B29874BC65EFBB775AB3BFBA837`；
  静默安装退出码 0，独立目录主程序 SHA 一致；安装目录原生模块存在，SHA-256
  `BAF2F84BE13E13B2026FCEB88DC09942728BA33F104A41FE70C1697E0A5CBCB3`。

◆ M3 地基批（2026-09-27，事实与回执）
- C4 失焦暂停已接线：`App.vue` 按 `blur`、`focus`、`visibilitychange` 维护
  `html.window-inactive`；`AmllBackground` 同步调用 MeshGradientRenderer 的
  `pause()` / `resume()`。歌词 rAF 未暂停，仍由音频时钟驱动。CSS 极光光带和分隔条
  在 `window-inactive` 下使用 `animation-play-state: paused`。
- 封面色彩管线新增纯数据边界：RGBA 固定降采样 64×64，使用
  `@material/material-color-utilities@0.4.0` 的 Celebi quantize、Score 和 HCT；
  输出中性明暗角色、彩色主角色、glow 和 onPrimary。低色度/空数据彩色角色回退
  `#4F8CF7`，不替用户作观感或验收判断。
- 玻璃基础加入 `thin/regular/thick` 三档 token/class：模糊 20/40/60px，饱和度
  1.4/1.6/1.8；不支持 `backdrop-filter` 或设置 `prefers-reduced-transparency` 时
  回退为实色主区背景。
- 回归：`npm run typecheck`、定向 ESLint、`npm test`（含 M3 地基接线检查）和
  `npm run build` 均退出码 0。影子三表沿用 M2 结果：scan 58/58、lyrics 4677/4677、
  settings_io 1551/1551，0 mismatch。
- Windows 构建与安装：`electron-builder --win` 退出码 0；安装器 97,765,778 B，
  SHA-256 `EFE2E491C343D86C5CD5D99B0DD3B0E1E2783E08F52E3BFD1B65B52C4D3C894A`；
  解包主程序及静默安装目录主程序 SHA-256 均为
  `625A230BCB82B35AF10AF39D51BF06A24324EEE78E0C3549BB0D5EB7AC90FB8F`；
  静默安装退出码 0；安装目录原生模块存在，SHA-256
  `BAF2F84BE13E13B2026FCEB88DC09942728BA33F104A41FE70C1697E0A5CBCB3`。

◆ M4 四季极光主题（2026-09-27，事实与回执）
- 新增春樱嫩绿、夏碧蓝青翠、秋橙绯红、冬冰蓝雪白四套极光主题；
  `aurora_seasonal_auto` 按本机日期在 3-5、6-8、9-11、12-2 月自动选择，
  本地午夜定时刷新；设置页同时提供手动入口。
- 主题切换过渡使用 `--anim-dur-theme: 0.28s` 与既有 easing tokens，
  对 `prefers-reduced-motion: reduce` 禁用过渡；主题切换 CSS 覆盖主壳、侧栏、
  视图、玻璃层和极光装饰。
- 回归：正式源码范围 ESLint、`npm run typecheck`、`npm test` 均通过；
  `npm run lint:gate` 的失败仅来自未跟踪 `work/cover-cdp-listener.mjs` 的 6 个
  显式返回类型错误，该临时脚本未纳入提交。
- Windows 构建与安装：`electron-builder --win` 退出码 0；安装器
  97,796,319 B，SHA-256
  `93224CD52CB1D36F96EB5749DD9F3FA48B05DCF7A9C47FE5FB8E65F84671F0F0`；
  解包主程序 211,232,768 B，SHA-256
  `240DF46B4487862CC5BA42556A31675139AB4DB6FD9A9C27E3782CD7ACF4A823`；
  原生模块 1,452,032 B，SHA-256
  `BAF2F84BE13E13B2026FCEB88DC09942728BA33F104A41FE70C1697E0A5CBCB3`。
  NSIS `/S` 静默安装退出码 0，安装目录主程序与原生模块哈希均一致。

◆ M5 F1 本地信息补全（2026-09-27，事实与回执）
- 新增本地信息补全任务：仅对能读到嵌入标题与艺术家标签的本地文件联网匹配，
  文件名回退值不会触发搜索；候选按规范化标题、艺术家和 5 秒时长容差筛选，
  歧义候选跳过。来源按网易云、酷我、酷狗、QQ 顺序尝试。
- 匹配到的封面 URL 和缺失专辑名回写曲库 song_json；边车/嵌入歌词或来源歌词
  写入既有 500 条 LRU 磁盘缓存。默认不改写音频文件，不把图片二进制写入 SQLite。
- 任务复用进度岛并可取消，最多 4 路并发；已完成的更新按 8 首一批回写并保留，
  取消不会进入未完成曲目。歌单右键新增“补全本地信息”。
- 回归：F1 纯逻辑匹配检查、`npm test`、`npm run typecheck`、正式文件定向
  ESLint 均退出码 0；未进行真实网络匹配，避免把外部服务结果当确定性回归数据。
- Windows 构建与安装：第一次临时输出目录的 7za 报错为既有
  `better-sqlite3\\prebuilds` 缺失路径警告，换全新 C 盘目录后
  `electron-builder --win` 退出码 0；安装器 97,827,349 B，SHA-256
  `B05A46BB0FD9059FC30F244FED235798BC9F38F4A94E791BC47137DE54356F3E`；
  解包主程序 211,232,768 B，SHA-256
  `9F906A13DC584A60B7338A53F8DABC8B75B565B701281C070402A1FE3C354076`；
  原生模块 1,452,032 B，SHA-256
  `BAF2F84BE13E13B2026FCEB88DC09942728BA33F104A41FE70C1697E0A5CBCB3`。
  NSIS `/S` 静默安装退出码 0，安装目录主程序与原生模块哈希均一致。

◆ M5 F1.5 ReplayGain 音量均衡（2026-09-27，事实与回执）
- 本地扫描从 Node `music-metadata` 与 Rust `lofty` 两条路径读取已有 ReplayGain
  track/album gain 与 peak 标签；字段以可选 `replayGain` 写入曲目 JSON，旧曲目、
  在线曲目和无标签文件不增加字段。此版本读取标签并应用，不对无标签音频做全曲
  LUFS 分析；重采样与写回标签留待 T3。
- 播放设置新增 ReplayGain 开关、单曲/专辑口径、前置增益和正增益上限；独立
  Web Audio GainNode 位于 IRS 合并后、淡入淡出输出前。峰值存在时限制正增益，
  无标签或关闭开关时保持 0 dB；切歌、恢复、试听和设置变更均平滑更新。
- 回归：`npm test`、`npm run typecheck`、Rust `cargo check`、`npm run native:compare`
  均通过；影子对答案 scan 58/58、lyrics 4677/4677、settings 1551/1551，真实
  文件校验一致。新增 M5.1 纯逻辑增益测试通过。
- Windows 构建与安装：先执行 `npm run native:build`，原生模块 SHA-256
  `0445C404C0598567FAAA749E217F5C735464CCF80A6CF6AAD08021DD8D60E2AA`；
  安装器 `dist/kunyin-desktop-1.0.7-setup.exe` 大小 `97,913,762 B`，SHA-256
  `ACA0DD01DBDB21D8E0D418F616FC4D865C79D27E31AAC529E61F234710AF96FB`；NSIS `/S`
  静默安装退出码 `0`。未下验收通过结论。

◆ M5 F3' 曲库管家（2026-09-27，事实与回执）
- 本地歌单右键新增 KUNYIN__ 文件名清理、查重与健康检查、撤销上次清理。
  清理先生成预览计划，只处理选中的本地歌单；Windows 非法字符、保留设备名、尾部点/空格和控制字符会被安全化，
  目标路径按 Windows 大小写与分隔符口径检测冲突，不自动删除文件。
- 执行改名后在 SQLite 事务内迁移本地歌曲 ID、歌单关联和重定向关联；文件改名或数据库迁移失败时尝试恢复原路径。
  撤销会检查新旧路径占用与曲库关联，撤销记录只保存在当前主进程内存，应用重启后不可用。
- 健康检查只把“文件大小相同且 SHA-256 相同”记为重复组；`music-metadata` 探测失败仅报告探测失败，不据此裁决音频完整性。
- 回归：`npm run test:m53`、`npm test`、`npm run typecheck` 均退出码 0；正式文件定向 ESLint 0 error（仅格式 warning）；`git diff --check` 通过。
- Windows 构建与安装：`npm run build:win` 退出码 0；安装器 `dist/kunyin-desktop-1.0.7-setup.exe` 大小 `97,881,638 B`，SHA-256
  `84E303CC5ECDA05C9136618C802981A234107E337864F6E2C938011F0B3DEB31`；
  解包主程序 211,232,768 B，SHA-256 `9222661A59060206C11C4F240289D21A3DE53A774E98C870E2181444B1115CE3`；
  独立临时目录 NSIS `/S` 安装后版本 `1.0.7.0`，主程序 SHA 与解包产物一致。未下验收通过结论。

◆ M5 F2 聚合搜索（2026-09-27，事实与回执）
- 搜索页新增“全部”入口。单曲聚合模式并发查询网易云、QQ、QQ 云、酷狗、酷我、JOOX 六个在线 Provider，按响应完成顺序追加来源分组；每组显示来源短标签、结果数、错误/无结果状态和独立“加载更多”。本地曲目不进入聚合请求。
- 搜索请求使用单调 generation。切换关键词、来源或搜索类型后，旧请求的结果、错误和 finally 状态不会覆盖当前查询；网络请求本身仍沿用既有 IPC/Provider 超时机制，没有新增跨层 AbortSignal。
- QQ 云返回的 `MusicItem.type` 可能为 `qq`，聚合展示的来源标签取 Provider 分组而非歌曲 type，避免来源误标。
- 新增 `runSourceSearches` 纯逻辑编排边界和 `test:m54` 时序测试，验证先回先显与旧查询结果作废。`npm run test:m54`、`npm test`、`npm run typecheck`、正式文件定向 ESLint 均通过；`git diff --check` 通过。
- Windows 构建与安装：`npm run build:win` 退出码 0；安装器 `dist/kunyin-desktop-1.0.7-setup.exe` 大小 `97,957,744 B`，SHA-256 `060C96E3D5763B6429BBC81C4E34D5357ABE2D7DB22A2D56DE96581CAFF26766`；独立临时目录 NSIS `/S` 退出码 0，安装版本 `1.0.7.0`，主程序 SHA-256 `465A8001BEDDD6789940E6CCBA597861EF4F5C4DEA796B5E6435D39713F0570F`。
