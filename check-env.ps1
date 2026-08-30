#Requires -Version 5.1
<#
.SYNOPSIS
    构建环境体检：只做检查，不构建、不改动任何文件。

.DESCRIPTION
    build-kunyin.ps1 闪退时先跑这个。它把每一项检查的结果打在屏幕上，
    并写进 check-env.log，最后停下来等回车，不会一闪而过。
#>
[CmdletBinding()]
param([switch]$NoPause)

$ErrorActionPreference = 'Continue'
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

$dir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
$log = Join-Path $dir 'check-env.log'
try { Start-Transcript -Path $log -Force | Out-Null } catch { }

function Line($k, $v, $color) { Write-Host ("{0,-24} {1}" -f "$k :", $v) -ForegroundColor $color }
function Ok($k, $v)   { Line $k $v 'Green' }
function Bad($k, $v)  { Line $k $v 'Red' }
function Info($k, $v) { Line $k $v 'Gray' }

Write-Host ''
Write-Host '===== kunyin-desktop 构建环境体检 =====' -ForegroundColor Cyan
Write-Host ''

Info 'PowerShell 版本' $PSVersionTable.PSVersion.ToString()
Info '当前进程执行策略' (Get-ExecutionPolicy -Scope Process)
Info '当前用户执行策略' (Get-ExecutionPolicy -Scope CurrentUser)
Info '本机执行策略' (Get-ExecutionPolicy -Scope LocalMachine)
Info '生效执行策略' (Get-ExecutionPolicy)
Info '交互模式' ([Environment]::UserInteractive)
Info '脚本所在目录' $dir
Write-Host ''

# 下载来的文件带「来源标记」，Unrestricted 策略下会弹确认框；
# 非交互启动时那个框没法确认，脚本就直接死掉——正是「闪退」的一种。
$self = Join-Path $dir 'build-kunyin.ps1'
if (Test-Path -LiteralPath $self) {
    Ok 'build-kunyin.ps1' '存在'
    $zone = Get-Item -LiteralPath $self -Stream Zone.Identifier -ErrorAction SilentlyContinue
    if ($zone) {
        Bad '文件来源标记' '有（建议执行 Unblock-File 解除，见文末）'
    } else {
        Ok '文件来源标记' '无'
    }
} else {
    Bad 'build-kunyin.ps1' "不存在（找不到 $self）"
}

$pkg = Join-Path $dir 'package.json'
if (Test-Path -LiteralPath $pkg) {
    try {
        $name = (Get-Content -LiteralPath $pkg -Raw -Encoding UTF8 | ConvertFrom-Json).name
        Ok 'package.json' "存在，name = $name"
    } catch {
        Bad 'package.json' "存在但解析失败：$($_.Exception.Message)"
    }
} else {
    Bad 'package.json' '同目录下没有（脚本要放在源码根目录）'
}

# 与 build-kunyin.ps1 同一套候选顺序：PATH → 附带的 tools\node → 装了但没进 PATH 的位置
$candidates = @()
$node = Get-Command node -CommandType Application -ErrorAction SilentlyContinue
if ($node) {
    $nodeSrc = if ($node -is [array]) { $node[0].Source } else { $node.Source }
    $candidates += ,@($nodeSrc, 'PATH')
}
$bundled = Join-Path $dir 'tools\node\node.exe'
if (Test-Path -LiteralPath $bundled -PathType Leaf) { $candidates += ,@($bundled, '源码附带') }
foreach ($base in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:LOCALAPPDATA,
                    $env:NVM_SYMLINK, $env:NVM_HOME, $env:VOLTA_HOME)) {
    if (-not $base) { continue }
    foreach ($rel in @('nodejs\node.exe', 'node.exe', 'Programs\nodejs\node.exe', 'bin\node.exe')) {
        $probe = Join-Path $base $rel
        if (Test-Path -LiteralPath $probe -PathType Leaf) { $candidates += ,@($probe, '已安装但不在 PATH') }
    }
}

$nodePath = $null
$nodeNote = $null
$tooOld = $null
foreach ($c in $candidates) {
    try { $nv = (& $c[0] --version 2>$null).Trim() } catch { continue }
    if (-not $nv -or $nv -notmatch '^v\d+\.\d+\.\d+') { continue }
    $p = [version]($nv.TrimStart('v').Split('-')[0])
    $good = ($p -ge [version]'20.19.0' -and $p -lt [version]'21.0.0') -or ($p -ge [version]'22.12.0')
    if ($good) { $nodePath = $c[0]; $nodeNote = "$nv（$($c[1])）"; break }
    if (-not $tooOld) { $tooOld = "$nv（$($c[1])：$($c[0])）" }
}

if ($nodePath) {
    Ok 'Node' $nodeNote
} elseif ($tooOld) {
    Bad 'Node' "版本过低 $tooOld —— 需要 20.19+ 或 22.12+"
} elseif ($candidates) {
    Bad 'Node' '找到了 node.exe 但无法执行'
} else {
    Bad 'Node' '未安装，且源码里没有 tools\node（winget install OpenJS.NodeJS.LTS）'
}

# npm 取选中那份 node 的同目录版本，跟构建脚本保持一致
$npmSrc = $null
if ($nodePath) {
    $sib = Join-Path (Split-Path -Parent $nodePath) 'npm.cmd'
    if (Test-Path -LiteralPath $sib -PathType Leaf) { $npmSrc = $sib }
}
if (-not $npmSrc) {
    $npm = Get-Command npm.cmd -CommandType Application -ErrorAction SilentlyContinue
    if (-not $npm) { $npm = Get-Command npm -CommandType Application -ErrorAction SilentlyContinue }
    if ($npm) { $npmSrc = if ($npm -is [array]) { $npm[0].Source } else { $npm.Source } }
}
if ($npmSrc) {
    try { Ok 'npm' ((& $npmSrc --version 2>$null).Trim() + "（$npmSrc）") }
    catch { Bad 'npm' "无法执行 $npmSrc" }
} else {
    Bad 'npm' '未找到（npm 随 Node 一起安装）'
}

if (Test-Path -LiteralPath (Join-Path $dir 'node_modules')) {
    Ok 'node_modules' '已存在（可加 -SkipInstall 省时间）'
} else {
    Info 'node_modules' '不存在（脚本会自动安装）'
}

try {
    $d = Get-PSDrive -Name $dir.Substring(0,1) -ErrorAction Stop
    $free = [math]::Round($d.Free / 1GB, 1)
    if ($d.Free -lt 3GB) { Bad '磁盘可用空间' "$free GB（建议 3GB 以上）" }
    else { Ok '磁盘可用空间' "$free GB" }
} catch { Info '磁盘可用空间' '无法读取' }

Write-Host ''
Write-Host '===== 体检结束 =====' -ForegroundColor Cyan
Write-Host ''
Write-Host '如果上面有红色项，先解决它再跑 build-kunyin.ps1。' -ForegroundColor Yellow
Write-Host '解除来源标记（如需要）：' -ForegroundColor Yellow
Write-Host '    Get-ChildItem *.ps1 | Unblock-File' -ForegroundColor Gray
Write-Host ''
Write-Host "本次记录：$log" -ForegroundColor DarkGray

try { Stop-Transcript | Out-Null } catch { }

if (-not $NoPause -and [Environment]::UserInteractive) {
    Write-Host '按回车键关闭窗口...' -ForegroundColor Cyan
    try { Read-Host | Out-Null } catch { Start-Sleep -Seconds 30 }
}
