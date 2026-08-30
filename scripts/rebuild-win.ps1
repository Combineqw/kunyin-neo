#requires -Version 5.1
[CmdletBinding()]
param(
  [switch]$SkipInstall,
  [switch]$SkipChecks
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$backupRoot = Join-Path $projectRoot ".workbuddy-dependency-repair"
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$nodeModules = Join-Path $projectRoot "node_modules"
$managedNode = "C:\Users\Administrator\.workbuddy\binaries\node\versions\22.22.2\node.exe"
$managedNpm = "C:\Users\Administrator\.workbuddy\binaries\node\versions\22.22.2\node_modules\npm\bin\npm-cli.js"

if (Test-Path $managedNode) {
  $node = $managedNode
} else {
  $node = (Get-Command node -ErrorAction Stop).Source
}

if (-not $SkipInstall) {
  if (Test-Path $nodeModules) {
    New-Item -ItemType Directory -Path $backupRoot -Force | Out-Null
    $backupPath = Join-Path $backupRoot "node_modules-$stamp"
    Write-Host "备份现有 node_modules 到: $backupPath"
    Move-Item -Path $nodeModules -Destination $backupPath -ErrorAction Stop
  }

  $env:NODE_TLS_REJECT_UNAUTHORIZED = "0"
  try {
    Write-Host "恢复 package-lock.json 锁定依赖..."
    if (Test-Path $managedNpm) {
      & $node $managedNpm --prefix $projectRoot ci
    } else {
      Push-Location $projectRoot
      try {
        & npm ci
      } finally {
        Pop-Location
      }
    }
    if ($LASTEXITCODE -ne 0) { throw "npm ci 失败，退出码: $LASTEXITCODE" }
  } finally {
    Remove-Item Env:NODE_TLS_REJECT_UNAUTHORIZED -ErrorAction SilentlyContinue
  }
}

if (-not $SkipChecks) {
  Write-Host "执行 TypeScript 与 Vue 类型检查..."
  Push-Location $projectRoot
  try {
    & npm run typecheck
    if ($LASTEXITCODE -ne 0) { throw "类型检查失败，退出码: $LASTEXITCODE" }

    Write-Host "执行推荐与 EQ 核心测试..."
    & npm run test:core
    if ($LASTEXITCODE -ne 0) { throw "核心测试失败，退出码: $LASTEXITCODE" }
  } finally {
    Pop-Location
  }
}

Write-Host "构建 Windows 安装包..."
Push-Location $projectRoot
try {
  & npm run build:win
  if ($LASTEXITCODE -ne 0) { throw "Windows 构建失败，退出码: $LASTEXITCODE" }
} finally {
  Pop-Location
}

$packages = Get-ChildItem -Path (Join-Path $projectRoot "dist") -Filter "*setup*.exe" -File -ErrorAction SilentlyContinue
if (-not $packages) { throw "构建完成但未在 dist 中找到安装包" }
$packages | Sort-Object LastWriteTime -Descending | Select-Object -First 1 | ForEach-Object {
  Write-Host "安装包已生成: $($_.FullName)"
}
