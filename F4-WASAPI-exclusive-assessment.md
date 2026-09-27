# F4' WASAPI 独占输出评估

日期：2026-09-27
范围：架构与实施评估；本阶段不修改播放器实现。

## 结论

Windows WASAPI 独占输出在技术上可实现，但当前播放器没有可直接启用它的接口。播放器音频从 Electron renderer 的 HTMLAudioElement 进入 Chromium Web Audio，经过 EQ、SRS、IRS、ReplayGain、淡入淡出和频谱分析后到达 `AudioContext.destination`。这条路径没有公开的应用级控制来要求 WASAPI `AUDCLNT_SHAREMODE_EXCLUSIVE`，也没有证据表明当前输出已独占或位深/采样率直通。

因此，F4' 不应作为现有播放器的一个简单设置开关实装。建议保留现有 Chromium 共享输出作为默认路径；若独占输出仍是产品目标，先做隔离的原生设备探测，再由用户决定是否进入播放后端迁移。不得把“成功打开独占设备”描述成整条播放链已位元直通。

## 当前实现边界

- `src/renderer/src/stores/player.ts` 创建唯一 `HTMLAudioElement`，经主进程注册的 `kunyin://` 地址播放本地或在线音频。
- `src/main/audio/protocol.ts` 负责本地文件与远端流读取、Range seek、上游鉴权请求及 QQ 加密流解密；媒体编解码交由 Chromium。
- `src/renderer/src/audio/audioGraph.ts` 使用 `createMediaElementSource()` 接入 Web Audio，随后运行均衡器、SRS、IRS、ReplayGain、淡入淡出与分析器，最终连接至 `context.destination`。
- `crates/aurora-native` 目前是提供纯函数的 N-API Rust 模块，没有音频解码器、输出设备枚举或 WASAPI render thread。
- 仓库没有播放器输出设备选择或 WASAPI 模式设置。Web Audio 的采样率/输出设备能力不能等同于 WASAPI 独占，也不能保证源文件编码位深原样送达设备。

## 可选路线

| 路线 | 做法 | 优点 | 主要代价与限制 |
| --- | --- | --- | --- |
| 原生后端接管 | Rust 解码输入并管理 WASAPI exclusive render thread；对各源提供 PCM | 架构上能直接协商设备格式，是追求格式直通的可信路线 | 需要接管本地和在线解码、Range/seek、鉴权、QQ 解密、格式覆盖、队列、淡入淡出、EQ/SRS/IRS/ReplayGain、频谱和设备热插拔；改动面最大 |
| Web Audio PCM 桥接原型 | AudioWorklet 将处理后 PCM 分块送到原生 WASAPI 输出线程 | 可保留 Chromium 解码和现有效果链，适合验证端点与独占打开 | 实时线程不能依赖 JS/N-API 同步工作；跨线程缓冲、欠载、延迟、时钟漂移和格式协商需要专门设计；Web Audio 重采样/效果仍意味着不是 bit-perfect |
| 不实装 | 继续使用 Chromium 共享输出，记录为暂缓或不做 | 保持现有播放兼容性与回归面 | 不提供独占端点控制或位深/采样率直通 |

`cpal` 等跨平台流式输出库本身不代表 WASAPI exclusive 支持。若进入原生路线，需明确使用公开暴露 `IAudioClient::Initialize` 独占模式的 Windows 后端，并验证设备协商与 HRESULT 错误处理；不能仅因加入 Rust 音频依赖就宣称独占。

## 推荐验证顺序

1. 先做不接播放器的原生端点探测：枚举输出设备，针对用户选定端点尝试独占打开/关闭，报告设备支持格式、实际协商格式和错误码。不得改变系统默认设备或持续占用设备。
2. 记录失败行为：设备不支持独占、被其他程序占用、格式不兼容、设备断开时，必须返回明确原因并释放资源。产品集成前决定是否自动回退共享输出，默认应保留现有播放器可用。
3. 只有端点探测结果满足目标设备后，才评估 PCM 桥接原型。测量 buffer 欠载、延迟、CPU、seek/切歌/暂停恢复及 165 Hz UI 干扰；不得称其为 bit-perfect。
4. 若目标是所有在线与本地源均独占且格式直通，按完整原生播放后端估算和拆分工作；逐项验证本地格式、各 Provider 流、QQ 解密、seek、DSP 与 shared fallback。

## 必须先拍板的产品行为

- F4' 范围是仅做 WAV/FLAC 本地文件的实验，还是要求本地和所有在线音源统一支持？
- 独占失败或设备被占用时，是自动回到共享输出，还是提示用户并停止播放？
- 开启 EQ、SRS、IRS 或 ReplayGain 时，是否允许 PCM 被处理/重采样？若允许，就不能将该模式称为位元直通。
- 独占时 Windows 系统提示音及其他应用无法共享该设备，是否可接受？

## 评估依据

- `src/renderer/src/stores/player.ts`：唯一 HTMLAudioElement 与播放状态机。
- `src/renderer/src/audio/audioGraph.ts`：Web Audio 图及 `context.destination` 输出。
- `src/main/audio/protocol.ts`：流式输入、Range 与解密边界。
- `crates/aurora-native/Cargo.toml`、`src/main/native/bridge.ts`：现有 Rust 模块为扫描/歌词/设置类 N-API 接口，不是音频后端。
- `MEMORY.md` 将 F4' 定义为评估制，并把 WASAPI、位深/采样率直通放入 T3 音频管线阶段。

本报告只记录技术可行性与实施边界，不构成用户验收结论，也未启用或改变任何音频输出行为。
