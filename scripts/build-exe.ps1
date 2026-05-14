#!/usr/bin/env pwsh
# ============================================================
#  凌台 Loft —— 一键生产构建脚本
# ============================================================
#
# 用法：
#   .\scripts\build-exe.ps1                # 便携 EXE（默认，最快）
#   .\scripts\build-exe.ps1 -Installer     # 完整安装包（MSI + NSIS）
#   .\scripts\build-exe.ps1 -Open          # 编译完后打开输出目录
#   .\scripts\build-exe.ps1 -Clean         # 清掉前端 dist 后重新构建
#
# 也可以走 pnpm：
#   pnpm exe                # 便携 EXE
#   pnpm exe:installer      # 安装包
# ------------------------------------------------------------

param(
    [switch]$Installer = $false,
    [switch]$Open = $false,
    [switch]$Clean = $false
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $repoRoot

# ------------------------------------------------------------
# 工具函数
# ------------------------------------------------------------
function Step([string]$msg) {
    Write-Host ""
    Write-Host "▸ $msg" -ForegroundColor Cyan
}
function Ok([string]$msg) {
    Write-Host "  ✓ $msg" -ForegroundColor Green
}
function Warn([string]$msg) {
    Write-Host "  ⚠ $msg" -ForegroundColor Yellow
}
function Fail([string]$msg) {
    Write-Host "  ✗ $msg" -ForegroundColor Red
    exit 1
}
function FmtSize([long]$bytes) {
    if ($bytes -ge 1GB) { return "{0:N2} GB" -f ($bytes / 1GB) }
    if ($bytes -ge 1MB) { return "{0:N2} MB" -f ($bytes / 1MB) }
    if ($bytes -ge 1KB) { return "{0:N1} KB" -f ($bytes / 1KB) }
    return "$bytes B"
}

# ------------------------------------------------------------
# Banner
# ------------------------------------------------------------
Write-Host ""
Write-Host "  ╔════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "  ║       凌台 · Loft —— 生产构建              ║" -ForegroundColor Green
Write-Host "  ║       $(if ($Installer) { '安装包模式 (MSI + NSIS)        ' } else { '便携 EXE 模式 (单文件 loft.exe)' })      ║" -ForegroundColor Green
Write-Host "  ╚════════════════════════════════════════════╝" -ForegroundColor Green

# ------------------------------------------------------------
# 工具链检查
# ------------------------------------------------------------
Step "检查工具链"
$required = @('node', 'pnpm', 'cargo')
foreach ($t in $required) {
    $cmd = Get-Command $t -ErrorAction SilentlyContinue
    if (-not $cmd) { Fail "缺少 $t，请先安装" }
    $v = & $t --version 2>&1 | Select-Object -First 1
    Ok "$t $v"
}

# ------------------------------------------------------------
# 清理（可选）
# ------------------------------------------------------------
if ($Clean) {
    Step "清理 dist/ 和 dist-exe/"
    foreach ($d in @('dist', 'dist-exe')) {
        if (Test-Path $d) {
            Remove-Item -Recurse -Force $d
            Ok "已删除 $d"
        }
    }
}

# ------------------------------------------------------------
# 1. 依赖
# ------------------------------------------------------------
Step "1/4  安装/校验 npm 依赖"
if (-not (Test-Path 'node_modules')) {
    pnpm install
    if ($LASTEXITCODE -ne 0) { Fail "pnpm install 失败" }
    Ok "依赖已安装"
} else {
    Ok "node_modules 已就绪（跳过安装）"
}

# ------------------------------------------------------------
# 2. 生成图标
# ------------------------------------------------------------
Step "2/4  生成图标"
node scripts/gen-icons.mjs
if ($LASTEXITCODE -ne 0) { Fail "生成图标失败" }

# ------------------------------------------------------------
# 3 + 4. 编译
# ------------------------------------------------------------
if ($Installer) {
    Step "3/4  打包前端"
    pnpm build
    if ($LASTEXITCODE -ne 0) { Fail "前端打包失败" }
    Ok "前端 dist 就绪"

    Step "4/4  Tauri 生产构建（含 MSI + NSIS 安装包）"
    pnpm tauri:build
    if ($LASTEXITCODE -ne 0) { Fail "Tauri 构建失败" }

    Write-Host ""
    Write-Host "  ╔════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "  ║   ✅ 构建完成！                            ║" -ForegroundColor Green
    Write-Host "  ╚════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""

    $bundle = "src-tauri\target\release\bundle"
    if (Test-Path $bundle) {
        Write-Host "  构建产物：" -ForegroundColor Cyan
        Get-ChildItem $bundle -Recurse -Include '*.exe', '*.msi' | ForEach-Object {
            Write-Host ("    {0,-12} {1}" -f (FmtSize $_.Length), $_.FullName)
        }
        if ($Open) { Invoke-Item $bundle }
    }
} else {
    Step "3/4  Tauri 构建（跳过安装包，仅生成 loft.exe）"
    # 走官方 tauri build --no-bundle，确保 release 模式下嵌入前端资源、
    # 避免 cargo 缓存的 dev 配置导致 exe 仍指向 localhost:1420
    pnpm exec tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) { Fail "tauri build 失败" }

    Step "4/4  拷贝 EXE 到 dist-exe/"
    if (-not (Test-Path 'dist-exe')) { New-Item -ItemType Directory 'dist-exe' | Out-Null }
    $exeSrc = 'src-tauri\target\release\loft.exe'
    if (-not (Test-Path $exeSrc)) { Fail "找不到 loft.exe（位置：$exeSrc）" }
    $exeDst = 'dist-exe\Loft.exe'
    Copy-Item $exeSrc $exeDst -Force

    $finalExe = Resolve-Path $exeDst
    $size = FmtSize ((Get-Item $finalExe).Length)

    Write-Host ""
    Write-Host "  ╔════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "  ║   ✅ 便携 EXE 已生成                       ║" -ForegroundColor Green
    Write-Host "  ╚════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""
    Write-Host "    路径   $finalExe" -ForegroundColor Cyan
    Write-Host "    大小   $size" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  双击直接运行，无需安装。"
    Write-Host "  首次启动如缺 WebView2 Runtime，Windows 会自动弹窗安装。"
    Write-Host ""

    if ($Open) { Invoke-Item 'dist-exe' }
}
