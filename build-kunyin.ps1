#Requires -Version 5.1
<#
.SYNOPSIS
    坤音neo (kunyin-desktop) Windows 构建脚本。

.DESCRIPTION
    在 Windows 上把源码构建成安装包。依次执行：环境检查 → 依赖安装 →
    可选类型检查 → electron-vite 编译 → electron-builder 打包。

    在 Windows 上构建不需要 wine（Linux/macOS 交叉构建 NSIS 才需要）。

.PARAMETER Path
    项目根目录（含 package.json）。默认为脚本所在目录。

.PARAMETER Target
    产物类型：
      setup    NSIS 向导式安装包（默认，即 npm run build:win）
      portable 单文件免安装 exe
      zip      解压即用的压缩包
      dir      只解包到 dist\win-unpacked，不封装（最快，用于验证）

.PARAMETER Arch
    目标架构：x64（默认）或 arm64。

.PARAMETER TypeCheck
    打包前跑 tsc / vue-tsc 类型检查。

.PARAMETER Verify
    打包前跑完整验收：ESLint + 类型检查 + 核心域测试 + 主题配色 + 动效约束
    （等价于 npm run verify）。比 -TypeCheck 更严，任何一项不达标就中止打包，
    不会产出「看起来成功」的安装包。

.PARAMETER Clean
    构建前删除 out\ 与 dist\。加 -Purge 时连 node_modules\ 一起删。

.PARAMETER Purge
    与 -Clean 连用，一并删除 node_modules 并重装依赖。

.PARAMETER SkipInstall
    跳过依赖安装（node_modules 已就绪时可省几分钟）。

.PARAMETER NoPause
    结束后不等待回车直接退出。给 CI / 自动化用；手工双击运行时别加，
    否则窗口会在脚本结束的瞬间关闭，什么都看不到。

.EXAMPLE
    .\build-kunyin.ps1
    出 dist\kunyin-desktop-1.0.7-setup.exe

.EXAMPLE
    .\build-kunyin.ps1 -Target portable -TypeCheck
    先类型检查，再出单文件免安装 exe

.EXAMPLE
    .\build-kunyin.ps1 -Verify
    跑完整验收（类型检查 + 三组测试）再出安装包。改过代码后建议用这个。

.EXAMPLE
    .\build-kunyin.ps1 -Clean -Purge
    彻底重来：清空 out/dist/node_modules 后完整构建
#>
[CmdletBinding()]
param(
    [string]$Path = $PSScriptRoot,
    [ValidateSet('setup', 'portable', 'zip', 'dir')]
    [string]$Target = 'setup',
    [ValidateSet('x64', 'arm64')]
    [string]$Arch = 'x64',
    [switch]$TypeCheck,
    [switch]$Verify,
    [switch]$Clean,
    [switch]$Purge,
    [switch]$SkipInstall,
    [switch]$NoPause
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# 控制台按 UTF-8 输出，避免 npm/electron-builder 的中文日志变成乱码
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

$script:StartedAt = Get-Date

# $PSScriptRoot 在「powershell -Command」这类调用下会是空的，兜个底
$script:ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }

# 全程写一份日志。窗口万一还是关掉了，至少有东西可查。
$script:LogPath = Join-Path $script:ScriptDir 'build-kunyin.log'
$script:Transcribing = $false
try {
    Start-Transcript -Path $script:LogPath -Force | Out-Null
    $script:Transcribing = $true
}
catch {
    Write-Host "（日志无法写入 $script:LogPath，继续构建）" -ForegroundColor DarkGray
}

function Stop-Log {
    if ($script:Transcribing) {
        try { Stop-Transcript | Out-Null } catch { }
        $script:Transcribing = $false
    }
}

# 双击 / 右键运行时，窗口会在脚本结束的瞬间关闭。这里统一在退出前停一下，
# 让成功信息和错误信息都能被看到。-NoPause 或非交互场景（CI）不停。
function Invoke-ExitPause {
    if ($NoPause) { return }
    if (-not [Environment]::UserInteractive) { return }
    Write-Host ''
    Write-Host "完整日志：$script:LogPath" -ForegroundColor DarkGray
    Write-Host '按回车键关闭窗口...' -ForegroundColor Cyan
    try { Read-Host | Out-Null } catch { Start-Sleep -Seconds 30 }
}

function Exit-Build([int]$Code) {
    Stop-Log
    Invoke-ExitPause
    exit $Code
}

function Write-Step([string]$Message) {
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}
function Write-Ok([string]$Message) { Write-Host "    $Message" -ForegroundColor Green }
function Write-Warn2([string]$Message) { Write-Host "    $Message" -ForegroundColor Yellow }

function Stop-Build([string]$Message, [string]$Hint) {
    Write-Host ''
    Write-Host "构建失败：$Message" -ForegroundColor Red
    if ($Hint) { Write-Host "处理建议：$Hint" -ForegroundColor Yellow }
    Exit-Build 1
}

# npm / npx 把进度和告警写进 stderr，PowerShell 会把 stderr 当错误。
# 所以这里统一用退出码判断成败，而不是靠 $ErrorActionPreference。
function Invoke-External {
    param(
        [Parameter(Mandatory)][string]$File,
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][string]$Activity
    )
    Write-Host "    > $File $($Arguments -join ' ')" -ForegroundColor DarkGray
    $global:LASTEXITCODE = 0
    if ($File -like '*.ps1') { & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $File @Arguments }
    else { & $File @Arguments }
    $code = $LASTEXITCODE
    if ($code -ne 0) { Stop-Build "$Activity 失败（退出码 $code）" '' }
}

function Resolve-Cmd([string]$Name) {
    $cmd = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue
    if ($cmd -is [array]) { return $cmd[0].Source }
    if ($cmd) { return $cmd.Source }
    return $null
}

# 兜住任何没预料到的终止性错误。没有这个 trap，脚本会带着一句红字直接结束，
# 双击运行的窗口跟着关掉，用户只看到「闪一下」。
trap {
    Write-Host ''
    Write-Host "脚本异常终止：$($_.Exception.Message)" -ForegroundColor Red
    if ($_.InvocationInfo -and $_.InvocationInfo.ScriptLineNumber) {
        Write-Host "位置：第 $($_.InvocationInfo.ScriptLineNumber) 行" -ForegroundColor DarkGray
    }
    Exit-Build 1
}

# ---------------------------------------------------------------- 1. 环境检查
Write-Step '检查构建环境'

$projectRoot = (Resolve-Path -LiteralPath $Path -ErrorAction SilentlyContinue)
if (-not $projectRoot) { Stop-Build "目录不存在：$Path" '用 -Path 指定源码根目录。' }
$projectRoot = $projectRoot.Path
$pkgPath = Join-Path $projectRoot 'package.json'
if (-not (Test-Path -LiteralPath $pkgPath)) {
    # 常见情形：脚本和解压出来的源码文件夹并排放着，而不是在它里面。
    # 往下找一层，命中 kunyin-desktop 就直接用，省得报错让人去猜。
    $found = Get-ChildItem -LiteralPath $projectRoot -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName 'package.json' } |
        Where-Object { Test-Path -LiteralPath $_ } |
        Where-Object {
            try {
                (Get-Content -LiteralPath $_ -Raw -Encoding UTF8 | ConvertFrom-Json).name -eq 'kunyin-desktop'
            }
            catch { $false }
        } |
        Select-Object -First 1

    if (-not $found) {
        Stop-Build "$projectRoot 下没有 package.json" '把脚本放进源码根目录（package.json 旁边），或用 -Path 指定，例如：.\build-kunyin.ps1 -Path C:\src\kunyin-desktop-1.0.7-src'
    }
    $pkgPath = $found
    $projectRoot = (Split-Path -Parent $found)
    Write-Warn2 "脚本不在源码根目录，已自动定位到：$projectRoot"
}

$pkg = Get-Content -LiteralPath $pkgPath -Raw -Encoding UTF8 | ConvertFrom-Json
# StrictMode 下访问不存在的属性会抛错，所以先探测再取值
function Get-PkgField([string]$Name) {
    if ($pkg.PSObject.Properties.Name -contains $Name) { return $pkg.$Name }
    return $null
}
$pkgName = Get-PkgField 'name'
$pkgVersion = Get-PkgField 'version'
if ($pkgName -ne 'kunyin-desktop') {
    Write-Warn2 "package.json 里的名字是 '$pkgName'，不是 kunyin-desktop，请确认目录选对了。"
}
Write-Ok "项目：$pkgName v$pkgVersion"
Write-Ok "目录：$projectRoot"

# vite 7 要求 Node ^20.19.0 || >=22.12.0，光看主版本号不够（20.5、22.11 都会漏过）
function Test-NodeVersion([string]$Exe) {
    try { $v = (& $Exe --version 2>$null).Trim() } catch { return $null }
    if (-not $v -or $v -notmatch '^v\d+\.\d+\.\d+') { return $null }
    $p = [version]($v.TrimStart('v').Split('-')[0])
    $ok = ($p -ge [version]'20.19.0' -and $p -lt [version]'21.0.0') -or ($p -ge [version]'22.12.0')
    return [pscustomobject]@{ Exe = $Exe; Version = $v; Parsed = $p; Ok = $ok }
}

# 候选按优先级排：PATH 里的 → 随源码附带的 tools\node → 装了但没进 PATH 的标准位置。
# 附带的那份是官方 node-v22.23.2-win-x64.zip 原样解包，免安装、不写注册表、不需要管理员。
$nodeCandidates = @()
$fromPath = Resolve-Cmd 'node'
if ($fromPath) { $nodeCandidates += $fromPath }
$bundledNode = Join-Path $projectRoot 'tools\node\node.exe'
if (Test-Path -LiteralPath $bundledNode -PathType Leaf) { $nodeCandidates += $bundledNode }
foreach ($base in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:LOCALAPPDATA,
                    $env:NVM_SYMLINK, $env:NVM_HOME, $env:VOLTA_HOME)) {
    if (-not $base) { continue }
    foreach ($rel in @('nodejs\node.exe', 'node.exe', 'Programs\nodejs\node.exe', 'bin\node.exe')) {
        $probe = Join-Path $base $rel
        if (Test-Path -LiteralPath $probe -PathType Leaf) { $nodeCandidates += $probe }
    }
}

$nodePick = $null
$rejected = @()
foreach ($cand in ($nodeCandidates | Select-Object -Unique)) {
    $info = Test-NodeVersion $cand
    if (-not $info) { continue }
    if ($info.Ok) { $nodePick = $info; break }
    $rejected += $info
}

if (-not $nodePick) {
    if ($rejected) {
        $r = $rejected[0]
        Stop-Build "Node 版本不满足要求：$($r.Version)（$($r.Exe)）" 'vite 7 需要 20.19+ 或 22.12+。源码里附带了 tools\node（v22.23.2），删掉本机旧版或直接用附带的即可。'
    }
    Stop-Build '找不到可用的 node' "源码里应附带 tools\node\node.exe。若已删除，装 Node 22 LTS：winget install OpenJS.NodeJS.LTS"
}

$nodeExe = $nodePick.Exe
$nodeVersion = $nodePick.Version
$nodeParsed = $nodePick.Parsed
$nodeDir = Split-Path -Parent $nodeExe

# 无论选中哪份都把它的目录顶到 PATH 最前：npm、electron-builder、node-gyp 都会再去
# spawn node / npm，它们查的是 PATH，光知道 node.exe 在哪不够。
$env:PATH = "$nodeDir;$env:PATH"

if ($nodeExe -eq $bundledNode) {
    Write-Ok "Node：$nodeVersion（源码附带的免安装版，未改动本机环境）"
}
elseif ($fromPath -and $nodeExe -eq $fromPath) {
    Write-Ok "Node：$nodeVersion"
}
else {
    Write-Ok "Node：$nodeVersion（$nodeDir，不在 PATH 中，本次临时使用）"
}
if ($nodeParsed.Major % 2 -ne 0) {
    Write-Warn2 "Node $nodeVersion 是非 LTS 版本，建议换 22 LTS。"
}

# npm 必须取选中那份 node 的同目录版本：跨版本混用（新 node + 旧 npm）会出各种怪问题
$npmCmd = $null
$npmSibling = Join-Path $nodeDir 'npm.cmd'
if (Test-Path -LiteralPath $npmSibling -PathType Leaf) { $npmCmd = $npmSibling }
if (-not $npmCmd) { $npmCmd = Resolve-Cmd 'npm.cmd' }
if (-not $npmCmd) { $npmCmd = Resolve-Cmd 'npm' }
if (-not $npmCmd) { Stop-Build '找不到 npm' '重装 Node.js（npm 随 Node 一起安装），或恢复源码里的 tools\node 目录。' }
Write-Ok "npm：$((& $npmCmd --version).Trim())"

if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -and $Arch -eq 'x64') {
    Write-Warn2 '当前是 ARM64 机器却在构建 x64 产物，如需原生版本请加 -Arch arm64。'
}

# NSIS 的构建缓存约 100MB，Electron 发行包解包后约 300MB，留 3GB 余量
$drive = Get-PSDrive -Name $projectRoot.Substring(0, 1) -ErrorAction SilentlyContinue
if ($drive -and $drive.Free -lt 3GB) {
    Write-Warn2 "$($drive.Name): 盘可用空间 $([math]::Round($drive.Free / 1GB, 1))GB，打包建议留 3GB 以上。"
}

# 长路径：Electron 依赖树很深，路径超 260 字符时 npm 会报 ENAMETOOLONG
if ($projectRoot.Length -gt 90) {
    Write-Warn2 "项目路径较深（$($projectRoot.Length) 字符），若报 ENAMETOOLONG 请移到更浅的目录，如 C:\src\kunyin。"
}

# 临时/会话目录：这类目录会被清理，dist\ 里的安装包可能连带消失
if ($projectRoot -match 'local-agent-mode-sessions|\\Temp\\|\\AppData\\Local\\Temp') {
    Write-Warn2 '项目在临时/会话目录里，该目录可能被清理，产物会一起消失。'
    Write-Warn2 '建议先整个文件夹移到 C:\src\ 之类的固定位置再构建。'
}

Push-Location -LiteralPath $projectRoot
try {

    # ------------------------------------------------------------ 2. 清理
    if ($Clean) {
        Write-Step '清理旧产物'
        $targets = @('out', 'dist')
        if ($Purge) { $targets += 'node_modules' }
        foreach ($dir in $targets) {
            $full = Join-Path $projectRoot $dir
            if (Test-Path -LiteralPath $full) {
                Write-Host "    删除 $dir\ ..." -ForegroundColor DarkGray
                Remove-Item -LiteralPath $full -Recurse -Force -ErrorAction Stop
                Write-Ok "已删除 $dir\"
            }
        }
        if (-not $Purge) { Write-Ok 'node_modules 保留（加 -Purge 可一并删除）' }
    }
    elseif ($Purge) {
        Write-Warn2 '-Purge 需要与 -Clean 连用，本次忽略。'
    }

    # ------------------------------------------------------------ 3. 依赖安装
    $nodeModules = Join-Path $projectRoot 'node_modules'
    if ($SkipInstall) {
        if (-not (Test-Path -LiteralPath $nodeModules)) {
            Stop-Build '指定了 -SkipInstall 但 node_modules 不存在' '去掉 -SkipInstall 让脚本装依赖。'
        }
        Write-Step '跳过依赖安装（-SkipInstall）'
    }
    else {
        Write-Step '安装依赖'
        Write-Host '    Electron 二进制约 100MB，首次安装耗时较久（.npmrc 已配 npmmirror 镜像）。' -ForegroundColor DarkGray
        # 有 package-lock.json 就用 ci：严格按锁文件装，可复现
        if (Test-Path -LiteralPath (Join-Path $projectRoot 'package-lock.json')) {
            Invoke-External $npmCmd @('ci') '依赖安装 (npm ci)'
        }
        else {
            Write-Warn2 '没有 package-lock.json，改用 npm install（版本可能与我这边不完全一致）。'
            Invoke-External $npmCmd @('install') '依赖安装 (npm install)'
        }
        Write-Ok '依赖安装完成'
    }

    # better-sqlite3 走预编译 N-API 二进制，缺了会在运行时报「找不到原生模块」
    $prebuildDir = Join-Path $nodeModules 'better-sqlite3\prebuilds'
    if (Test-Path -LiteralPath $prebuildDir) {
        $expected = "win32-$Arch.node"
        if (Test-Path -LiteralPath (Join-Path $prebuildDir $expected)) {
            Write-Ok "better-sqlite3 预编译二进制就位（$expected）"
        }
        else {
            Write-Warn2 "缺少 better-sqlite3\prebuilds\$expected，打出的包可能启动即崩。"
        }
    }

    # ------------------------------------------------------------ 4. 类型检查 / 验收
    if ($TypeCheck -or $Verify) {
        Write-Step '类型检查'
        Invoke-External $npmCmd @('run', 'typecheck:node') '主进程类型检查'
        Invoke-External $npmCmd @('run', 'typecheck:web') '渲染层类型检查'
        Write-Ok '类型检查通过'
    }

    if ($Verify) {
        # 用 lint:gate 而非 lint：仓库整树是 CRLF，而 prettier 默认按 LF 判，
        # 直接跑 lint 会刷出三万多条 ␍ 警告把真正的问题埋掉。--quiet 只报 error，
        # 退出码语义不变（有 error 才非零）。
        Write-Step '代码检查'
        Invoke-External $npmCmd @('run', 'lint:gate') 'ESLint 检查'
        Write-Ok '代码检查通过'

        # 三组测试都是纯计算、无 IO，跑完只要几秒；放在编译之前是为了让不达标
        # 尽早中止——没有必要为一个注定不该交付的版本等完整个打包流程。
        Write-Step '验收测试'
        Invoke-External $npmCmd @('run', 'test:core') '核心域测试'
        Invoke-External $npmCmd @('run', 'test:theme') '主题配色验收'
        Invoke-External $npmCmd @('run', 'test:animation') '动效约束验收'
        Write-Ok '验收测试全部通过'
    }

    # ------------------------------------------------------------ 5. 编译
    Write-Step '编译主进程 / preload / 渲染层（electron-vite build）'
    Invoke-External $npmCmd @('run', 'build') 'electron-vite 编译'
    foreach ($f in @('out\main\index.js', 'out\preload\index.js', 'out\renderer\index.html')) {
        if (-not (Test-Path -LiteralPath (Join-Path $projectRoot $f))) {
            Stop-Build "编译产物缺失：$f" '看上面 electron-vite 的报错。'
        }
    }
    Write-Ok '编译完成（out\main、out\preload、out\renderer 均已生成）'

    # ------------------------------------------------------------ 6. 打包
    $targetArgs = switch ($Target) {
        'setup'    { @('--win', 'nsis') }
        'portable' { @('--win', 'portable') }
        'zip'      { @('--win', 'zip') }
        'dir'      { @('--win', '--dir') }
    }
    $targetDesc = switch ($Target) {
        'setup'    { 'NSIS 向导式安装包' }
        'portable' { '单文件免安装 exe' }
        'zip'      { '免安装压缩包' }
        'dir'      { '仅解包（不封装）' }
    }

    Write-Step "打包：$targetDesc（$Arch）"
    if ($Target -eq 'setup') {
        Write-Host '    首次打包会下载 NSIS 工具链（约 30MB），已配镜像。' -ForegroundColor DarkGray
    }
    # 直接用本地 node_modules 里的 electron-builder，不走 npm exec：
    # npm exec 在本地找不到时会去网上现拉一个版本，那样和锁文件里的版本就不一致了。
    $builderCmd = $null
    foreach ($candidate in @('.bin\electron-builder.cmd', '.bin\electron-builder.ps1', '.bin\electron-builder')) {
        $full = Join-Path $nodeModules $candidate
        if (Test-Path -LiteralPath $full) { $builderCmd = $full; break }
    }
    if (-not $builderCmd) {
        Stop-Build 'node_modules\.bin 下找不到 electron-builder' '先让脚本装完依赖（去掉 -SkipInstall）。'
    }
    # 架构显式传入，避免跟随宿主机（ARM64 机器上默认会出 arm64 包）
    Invoke-External $builderCmd ($targetArgs + @("--$Arch")) 'electron-builder 打包'

    # ------------------------------------------------------------ 7. 产物汇总
    Write-Step '产物'
    $distDir = Join-Path $projectRoot 'dist'
    if (-not (Test-Path -LiteralPath $distDir)) { Stop-Build 'dist\ 没有生成' '看上面 electron-builder 的报错。' }

    $artifacts = Get-ChildItem -LiteralPath $distDir -File |
        Where-Object { $_.Extension -in @('.exe', '.zip', '.msi', '.blockmap') } |
        Where-Object { $_.Name -notlike '*__uninstaller*' } |
        Sort-Object Length -Descending

    if ($Target -eq 'dir') {
        $unpacked = Join-Path $distDir 'win-unpacked'
        if (Test-Path -LiteralPath $unpacked) {
            $size = (Get-ChildItem -LiteralPath $unpacked -Recurse -File | Measure-Object -Sum Length).Sum
            Write-Host ('    {0,-52} {1,8:N1} MB' -f 'dist\win-unpacked\', ($size / 1MB))
            Write-Ok "可直接运行：$(Join-Path $unpacked 'kunyin-desktop.exe')"
        }
    }
    elseif (-not $artifacts) {
        Stop-Build 'dist\ 里没找到安装包' '确认 electron-builder 日志末尾是否有 building 行。'
    }
    else {
        foreach ($a in $artifacts) {
            if ($a.Extension -eq '.blockmap') { continue }
            Write-Host ('    {0,-52} {1,8:N1} MB' -f $a.Name, ($a.Length / 1MB))
            $hash = (Get-FileHash -LiteralPath $a.FullName -Algorithm SHA256).Hash.ToLower()
            Write-Host "      sha256 $hash" -ForegroundColor DarkGray
        }
    }

    $elapsed = (Get-Date) - $script:StartedAt
    Write-Host ''
    Write-Host "构建完成，用时 $([int]$elapsed.TotalMinutes) 分 $($elapsed.Seconds) 秒。" -ForegroundColor Green
    Write-Host "产物目录：$distDir" -ForegroundColor Green
    if ($Target -eq 'setup') {
        Write-Host ''
        Write-Host '注意：安装包未做代码签名，首次运行 Windows 会弹 SmartScreen 提示，' -ForegroundColor Yellow
        Write-Host '选「更多信息 → 仍要运行」即可。要消除提示需自备代码签名证书。' -ForegroundColor Yellow
    }
}
finally {
    Pop-Location
}

Exit-Build 0
