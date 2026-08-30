/**
 * 应用设置（对应 Android 端 common/AppSettingsManager.kt 各分类）
 *
 * 采用嵌套对象结构（比 lx-music-desktop 的扁平点分 key 更适合 Pinia/TS）。
 * 敏感凭据不在此，走主进程 safeStorage 单独加密存储。
 */
import type { QualityId } from './music'
import type { EqualizerFilter, EqualizerProfileMeta } from './equalizer'
import type { IrsProfile } from './irs'
import { DEFAULT_CUSTOM_ANIMATION_PACK, type AnimationPack } from './animation'

export type PlayMode = 'order' | 'listLoop' | 'singleLoop' | 'random' | 'heartbeat'
export type EqualizerPresetId = 'flat' | 'bass' | 'vocal' | 'treble' | 'custom'

/** 界面亮度档位（护眼）。'standard' = 不改动主题原色。
 * 实现在渲染层 theme/comfort.ts：只压浅色主题的大面积亮面，不动字色。 */
export type ComfortLevel = 'standard' | 'soft' | 'softer'

/** 用户自定义主题（简化自 lx-music-desktop 的 ThemeEditModal 产物） */
/** 极光光带覆写色（映射到 --aurora-c1 / --aurora-c3 / --aurora-glow） */
export interface ThemeAuroraConfig {
  c1?: string
  c3?: string
  glow?: string
}

export interface CustomThemeConfig {
  id: string
  name: string
  /** 深色底模式 */
  isDark: boolean
  /** 主色 rgb(...) */
  primary: string
  /** 字色 rgb(...) */
  font: string
  /** 深色字体梯度（用于浅色底上的低对比辅助文字） */
  isDarkFont?: boolean
  /** 背景图绝对路径；空串表示无背景图 */
  bgImage: string
  /** 应用/侧栏背景色 */
  appBackground?: string
  /** 侧栏按钮颜色 */
  sidebarButton?: string
  /** 内容区域背景色 */
  contentBackground?: string
  /** 音质标签主色 */
  badgePrimary?: string
  /** 音质标签次要色 */
  badgeSecondary?: string
  /** 音质标签第三色 */
  badgeTertiary?: string
  /** 窗口控制按钮颜色 */
  buttonClose?: string
  buttonMin?: string
  buttonHide?: string
  /** 极光光带覆写（导入的主题文件可携带；值经 themeConfigFromJson 校验为纯色） */
  aurora?: ThemeAuroraConfig
}

export interface AppSettings {
  /** 设置结构版本号，用于迁移 */
  version: number

  appearance: {
    /** 主题 id；'auto' 表示跟随系统深浅色 */
    themeId: string
    followSystem: boolean
    /** followSystem / auto 模式下的浅色主题 */
    lightThemeId: string
    /** followSystem / auto 模式下的深色主题 */
    darkThemeId: string
    /** 用户自定义主题列表 */
    customThemes: CustomThemeConfig[]
    /** UI 语言 */
    lang: string
    /** 软件界面字体（字体族名，空=跟随系统默认字体栈） */
    appFont: string
    /** 窗口尺寸档位（WINDOW_SIZE_LIST 下标） */
    windowSizeId: number
    /** 界面字体大小 px（14~19，经 #app zoom 实现整体缩放） */
    fontSize: number
    /** 界面亮度档位（护眼）：standard 不改动，soft/softer 压暗浅色主题的大面积亮面 */
    comfortLevel: ComfortLevel
  }

  /** 桌面端窗口与动效行为（对应 lx-music-desktop 基础设置） */
  behavior: {
    /** 显示界面过渡与弹出层动画 */
    showAnimation: boolean
    /** 弹出层每次从若干入场动画中随机选择 */
    randomAnimation: boolean
    /** 启动后直接进入全屏 */
    startInFullscreen: boolean
    /** 点击关闭按钮时隐藏主窗口，由托盘继续驻留 */
    closeToTray: boolean
    /** 当前动效包：内置预设或 custom */
    animationPackId: import('./animation').AnimationPackId
    /** 自定义动效包配置，切换回 custom 时生效 */
    customAnimationPack: AnimationPack
  }

  /** 列表显示（对应 lx-music-desktop 基础设置的列表项） */
  list: {
    /** 显示列表操作按钮（歌曲行 hover 的试听/添加/下载） */
    showOperationButtons: boolean
    /** 歌手页专辑用列表视图（false=网格；对应 Android albumListMode） */
    albumListMode: boolean
    /** 歌手页专辑按发行时间升序排列（false=新→旧，true=旧→新） */
    albumSortAsc: boolean
  }

  player: {
    /** 音量 0..1 */
    volume: number
    playMode: PlayMode
    /** 首选音质 */
    preferredQuality: QualityId
    /** 启动是否自动续播 */
    autoPlay: boolean
    /** 是否启用十段均衡器 */
    equalizerEnabled: boolean
    /** 当前均衡器预设；手动修改频段后为 custom */
    equalizerPreset: EqualizerPresetId
    /** 十段均衡器增益，单位 dB，范围 -12..12 */
    equalizerGains: number[]
    /** 当前调音文件元数据；频段数据仍存于 equalizerGains */
    equalizerProfile: EqualizerProfileMeta
    /** 当前动态滤镜链；缺省时由十段增益兼容生成。 */
    equalizerFilters: EqualizerFilter[]
    /** 当前动态配置的独立前置增益，单位 dB。 */
    equalizerPreampDb: number
    /** 用户已命名保存的自定义调音预设。 */
    equalizerCustomProfiles: import('./equalizer').EqualizerProfile[]
    /** SRS 风格空间音效总开关。 */
    srsEnabled: boolean
    /** SRS 总强度 0..100；各分项增强量会按此比例缩放。 */
    srsIntensity: number
    /** 低音增强强度 0..100。 */
    srsBass: number
    /** 人声清晰度强度 0..100。 */
    srsVoice: number
    /** 高频空气感强度 0..100。 */
    srsTreble: number
    /** 立体声空间宽度 0..100，100 为最大扩展。 */
    srsSpace: number
    /** 启用输出保护，限制空间处理后的峰值增益。 */
    srsLimiter: boolean
    /** IRS 卷积总开关。 */
    irsEnabled: boolean
    /** IRS 湿声比例 0..100；干声比例为 100 - wet。 */
    irsWetPercent: number
    /** 旧版本干声比例字段，仅用于读取迁移，不作为新 UI 语义。 */
    irsDryPercent: number
    /** 已导入的 IRS 脉冲响应预设。 */
    irsProfiles: IrsProfile[]
    /** 当前 IRS 预设 id，空值表示未选择。 */
    irsProfileId: string
    /** 切歌、暂停和恢复播放时是否使用淡入淡出 */
    fadeEnabled: boolean
    /** 淡入淡出时长，单位 ms */
    fadeDurationMs: number
    /** 独立迷你播放器悬浮窗开关 */
    miniPlayerEnabled: boolean
    /** 迷你播放器窗口左上角位置（null 表示默认右下角） */
    miniPlayerX: number | null
    miniPlayerY: number | null
    dailyRecommendationEnabled: boolean
    heartbeatEnabled: boolean
    recommendationMaxPerArtist: number
  }

  /** 音质过滤（播放取流与下载共用） */
  quality: {
    /** 屏蔽 AI 生成音质：播放与下载都跳过 aiQualities 里的档位，歌曲行徽标也不再显示 */
    blockAi: boolean
    /** 视为「AI 音质」的档位（可选项见 AI_QUALITY_CANDIDATES） */
    aiQualities: QualityId[]
  }

  lyrics: {
    showTranslation: boolean
    showRomanization: boolean
    fontSize: number
    /** 歌词字体（字体族名，空=跟随软件字体） */
    font: string
    /** 桌面歌词窗口开关 */
    desktopEnabled: boolean
    /** 锁定后窗口点击穿透 */
    desktopLocked: boolean
    /** 窗口置顶 */
    desktopAlwaysOnTop: boolean
    /** 定时刷新置顶状态，避免被部分全屏程序覆盖 */
    desktopAlwaysOnTopLoop: boolean
    /** 在系统任务栏显示桌面歌词窗口 */
    desktopShowTaskbar: boolean
    /** 主窗口全屏时隐藏桌面歌词 */
    desktopFullscreenHide: boolean
    /** 暂停播放时淡出桌面歌词 */
    desktopPauseHide: boolean
    /** 显示音频可视化装饰 */
    desktopAudioVisualization: boolean
    /** 桌面歌词窗口位置（null 表示未设置，用默认居中底部） */
    desktopX: number | null
    desktopY: number | null
    /** 桌面歌词窗口尺寸 */
    desktopWidth: number
    desktopHeight: number
    /** 限制窗口留在当前屏幕工作区内 */
    desktopLockScreen: boolean
    /** 使用更舒缓的延迟滚动 */
    desktopDelayScroll: boolean
    /** 当前歌词在窗口中的滚动锚点 */
    desktopScrollAlign: 'top' | 'center'
    /** 鼠标划过窗口时降低歌词透明度 */
    desktopHoverHide: boolean
    /** 歌词排版方向 */
    desktopDirection: 'horizontal' | 'vertical'
    /** 歌词水平对齐 */
    desktopAlign: 'left' | 'center' | 'right'
    /** 桌面歌词字体（空=跟随歌词字体） */
    desktopFont: string
    /** 桌面歌词字号 */
    desktopFontSize: number
    /** 桌面歌词基础字重 */
    desktopFontWeight: number
    /** 歌词行间距 */
    desktopLineGap: number
    /** 桌面歌词已播放（高亮）文字颜色 */
    desktopColorActive: string
    /** 桌面歌词未播放文字颜色 */
    desktopColorNormal: string
    /** 文字描边颜色 */
    desktopStrokeColor: string
    /** 文字描边宽度 px（0=关闭） */
    desktopStrokeWidth: number
    /** 显示文字阴影 */
    desktopShadowEnabled: boolean
    /** 文字阴影水平偏移 px */
    desktopShadowX: number
    /** 文字阴影垂直偏移 px */
    desktopShadowY: number
    /** 文字阴影模糊半径 px */
    desktopShadowBlur: number
    /** 文字阴影不透明度 0..1 */
    desktopShadowOpacity: number
    /** 显示半透明黑色渐变底条 */
    desktopGradientBar: boolean
    /** 渐变底条中心区域宽度百分比 */
    desktopGradientWidth: number
    /** 渐变底条不透明度 0..1 */
    desktopGradientOpacity: number
    /** 歌词整体不透明度 6..100 */
    desktopOpacity: number
    /** 长歌词单行省略，不自动换行 */
    desktopEllipsis: boolean
    /** 放大当前播放行 */
    desktopZoomActive: boolean
    /** 加粗逐字歌词 */
    desktopBoldSyllable: boolean
    /** 加粗逐行歌词 */
    desktopBoldLine: boolean
    /** 加粗翻译和音译 */
    desktopBoldExtended: boolean
    /** 桌面歌词背景不透明度 0..1（0=全透明） */
    desktopBgOpacity: number
  }

  network: {
    proxy: {
      enable: boolean
      host: string
      port: number
    }
  }

  download: {
    /** 下载功能总开关（关闭后不再接收新任务） */
    enabled: boolean
    path: string
    /** 优先下载音质（单曲/批量下载弹窗的默认高亮与无指定时的回退首选） */
    preferredQuality: QualityId
    /** 整专下载单独保存一份专辑封面（v26.6.6 特性） */
    saveAlbumCover: boolean
    /** 文件名追加采样率/位深标记，如 [16Bit-44.1kHz]（仅无损，对应 Android downloadSampleRateTag） */
    appendQualityTag: boolean
    maxConcurrent: number
    /** 文件命名风格（对应 Android DownloadNamingStyle） */
    namingStyle: 'artist-title' | 'title-artist' | 'title-only'
    /** 整专下载的文件名前缀两位曲目号，如 `01.歌手 - 歌名`（对应 downloadTrackNumber） */
    trackNumberPrefix: boolean
    /** 同名文件是否覆盖（否则自动追加序号，对应 downloadOverwriteExisting） */
    overwriteExisting: boolean
    /** 存在同名文件时跳过下载（优先于 overwriteExisting；对应 download.skipExistFile） */
    skipExistFile: boolean
    /** 按歌单名分组保存（下载目录下再建歌单名子目录；对应 download.isSavePathGroupByListName） */
    groupByListName: boolean
    /** 把歌曲封面嵌入音频标签（对应 download.isEmbedPic） */
    embedCover: boolean
    /** 把歌词写入音频标签（对应 download.isEmbedLyric） */
    embedLyric: boolean
    /** 嵌入翻译歌词（需 embedLyric） */
    embedLyricT: boolean
    /** 嵌入罗马音歌词（需 embedLyric） */
    embedLyricR: boolean
    /** 嵌入逐字歌词（需 embedLyric） */
    embedLyricLx: boolean
    /** 额外保存 .lrc 歌词文件（对应 download.isDownloadLrc） */
    saveLrcFile: boolean
    /** 歌词文件附带翻译 */
    saveLrcT: boolean
    /** 歌词文件附带罗马音 */
    saveLrcR: boolean
    /** 歌词文件附带逐字歌词 */
    saveLrcLx: boolean
    /** 歌词文件编码（对应 download.lrcFormat） */
    lrcFormat: 'utf8' | 'gbk'
    /** 歌曲源不可用时换源下载（对应 download.isUseOtherSource） */
    useOtherSource: boolean
  }

  sync: {
    enable: boolean
    /** LX 同步服务端地址（http(s)://host:port/sync 或裸 host），对应 Android serverUrl */
    serverUrl: string
    /** 激活卡密（CDK），用于首次密钥协商 */
    cdk: string
    /** 本设备名（显示在服务端设备列表） */
    deviceName: string
    /** 同步模式：与 Go 端 TransMode 一致 */
    syncMode:
      | 'merge_local_remote'
      | 'merge_remote_local'
      | 'overwrite_local_remote'
      | 'overwrite_remote_local'
    /** 启动时自动连接 */
    autoConnect: boolean
  }
}

export const DEFAULT_SETTINGS: AppSettings = {
  version: 3,
  appearance: {
    themeId: 'green',
    followSystem: true,
    lightThemeId: 'green',
    darkThemeId: 'black',
    customThemes: [],
    lang: 'zh-cn',
    appFont: '',
    windowSizeId: 3,
    fontSize: 16,
    comfortLevel: 'standard'
  },
  behavior: {
    showAnimation: true,
    randomAnimation: true,
    startInFullscreen: false,
    closeToTray: false,
    animationPackId: 'ios',
    customAnimationPack: { ...DEFAULT_CUSTOM_ANIMATION_PACK }
  },
  list: {
    showOperationButtons: true,
    albumListMode: false,
    albumSortAsc: false
  },
  player: {
    volume: 1,
    playMode: 'listLoop',
    preferredQuality: 'flac',
    autoPlay: false,
    equalizerEnabled: false,
    equalizerPreset: 'flat',
    equalizerGains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    equalizerProfile: {
      format: 'kunyin-eq',
      version: 1,
      name: '原声',
      author: '坤音neo',
      source: 'local'
    },
    equalizerFilters: [],
    equalizerPreampDb: 0,
    equalizerCustomProfiles: [],
    srsEnabled: false,
    srsIntensity: 100,
    srsBass: 35,
    srsVoice: 20,
    srsTreble: 18,
    srsSpace: 24,
    srsLimiter: true,
    irsEnabled: false,
    irsWetPercent: 50,
    irsDryPercent: 50,
    irsProfiles: [],
    irsProfileId: '',
    fadeEnabled: true,
    fadeDurationMs: 300,
    miniPlayerEnabled: false,
    miniPlayerX: null,
    miniPlayerY: null,
    dailyRecommendationEnabled: true,
    heartbeatEnabled: true,
    recommendationMaxPerArtist: 2
  },
  quality: {
    blockAi: true,
    aiQualities: ['atmos', 'atmos_plus']
  },
  lyrics: {
    showTranslation: true,
    showRomanization: false,
    fontSize: 22,
    font: '',
    desktopEnabled: false,
    desktopLocked: false,
    desktopAlwaysOnTop: true,
    desktopAlwaysOnTopLoop: false,
    desktopShowTaskbar: false,
    desktopFullscreenHide: true,
    desktopPauseHide: false,
    desktopAudioVisualization: false,
    desktopX: null,
    desktopY: null,
    desktopWidth: 640,
    desktopHeight: 180,
    desktopLockScreen: true,
    desktopDelayScroll: true,
    desktopScrollAlign: 'center',
    desktopHoverHide: false,
    desktopDirection: 'horizontal',
    desktopAlign: 'left',
    desktopFont: '',
    desktopFontSize: 28,
    desktopFontWeight: 600,
    desktopLineGap: 36,
    desktopColorActive: '#E8C083',
    desktopColorNormal: '#F5F5F2',
    desktopStrokeColor: '#151515',
    desktopStrokeWidth: 0,
    desktopShadowEnabled: true,
    desktopShadowX: 0,
    desktopShadowY: 2,
    desktopShadowBlur: 6,
    desktopShadowOpacity: 0.72,
    desktopGradientBar: false,
    desktopGradientWidth: 72,
    desktopGradientOpacity: 0.52,
    desktopOpacity: 100,
    desktopEllipsis: false,
    desktopZoomActive: false,
    desktopBoldSyllable: true,
    desktopBoldLine: true,
    desktopBoldExtended: false,
    desktopBgOpacity: 0.28
  },
  network: {
    proxy: {
      enable: false,
      host: '',
      port: 0
    }
  },
  download: {
    enabled: true,
    path: '',
    preferredQuality: 'flac',
    saveAlbumCover: true,
    appendQualityTag: true,
    maxConcurrent: 3,
    namingStyle: 'artist-title',
    trackNumberPrefix: true,
    overwriteExisting: false,
    skipExistFile: false,
    groupByListName: false,
    embedCover: true,
    embedLyric: true,
    embedLyricT: false,
    embedLyricR: false,
    embedLyricLx: false,
    saveLrcFile: false,
    saveLrcT: false,
    saveLrcR: false,
    saveLrcLx: true,
    lrcFormat: 'utf8',
    useOtherSource: false
  },
  sync: {
    enable: false,
    serverUrl: 'https://c.wwwweb.top/sync',
    cdk: '',
    deviceName: 'KunYin Desktop',
    syncMode: 'merge_local_remote',
    autoConnect: false
  }
}

/**
 * 代理实际生效状态。setProxy 是 session 全局的，指向死端口的规则会让封面/取流/<audio>
 * 全部失败，故主进程应用前先探活；配了但不可达时 enabled=true 而 active=false。
 */
export interface ProxyStatus {
  /** 设置项里是否开启 */
  enabled: boolean
  /** 是否真正应用到了 session（探活通过） */
  active: boolean
  /** 未生效原因（enabled 为 true 而 active 为 false 时有值） */
  reason?: string
}

/** 各类缓存用量（设置页「缓存管理」展示） */
export interface CacheStats {
  /** 资源缓存字节数（封面等图片走 Chromium HTTP 磁盘缓存） */
  resourceBytes: number
  /** 播放地址缓存条目数 */
  urlCount: number
  /** 歌词缓存条目数 */
  lyricCount: number
}

/** 可单独清理的缓存类型 */
export type CacheKind = 'resource' | 'url' | 'lyric'

/** 卡密激活状态（authst 校验结果，主/渲染共享） */
export interface AuthState {
  /** 当前卡密（authst）；为空表示未激活 */
  authst: string
  /** 最近一次校验是否通过 */
  isValid: boolean
  /** 校验反馈信息 */
  message: string
}
