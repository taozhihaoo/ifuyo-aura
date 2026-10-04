# M11 §57 安装器 smoke test（真实 NSIS 安装 → 启动 → 退出 → 卸载）：
#   M11-INSTALL-01  静默安装成功（per-user，§3 无需管理员）
#   M11-INSTALL-02  已安装 exe 启动（窗口进程存活）
#   M11-INSTALL-03  卸载后应用目录移除
#   M11-INSTALL-04  用户数据（app data）在卸载后保留（§3/§4 卸载 ≠ 删
#                   用户数据；不触碰用户音乐）
# 前置：tauri build 产物存在于 target/release/bundle/nsis/。
param(
    [string]$Installer = ""
)
$ErrorActionPreference = "Continue"

$repo = Split-Path -Parent $PSScriptRoot
if ($Installer -eq "") {
    $cand = Get-ChildItem (Join-Path $repo "target/release/bundle/nsis") -Filter "*-setup.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $cand) { throw "installer not found（先 tauri build）" }
    $Installer = $cand.FullName
}
Write-Host "installer: $Installer"
Get-Process aura-shell -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

# ---- 01 静默安装（NSIS /S = per-user silent）----
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$p = Start-Process $Installer -ArgumentList "/S" -PassThru
while (-not $p.HasExited -and $sw.ElapsedMilliseconds -lt 120000) { Start-Sleep -Milliseconds 250 }
$installDir = Join-Path $env:LOCALAPPDATA "ifuyo Aura"
$exe = Join-Path $installDir "aura-shell.exe"
Write-Host ("M11-INSTALL-01 installer-exitcode: " + $p.ExitCode + " elapsed_ms=" + $sw.ElapsedMilliseconds)
Write-Host ("M11-INSTALL-01 installed-exe-exists: " + (Test-Path $exe))

# ---- 02 启动已安装应用 ----
if (Test-Path $exe) {
    $app = Start-Process $exe -PassThru
    Start-Sleep -Seconds 6
    $alive = -not $app.HasExited
    Write-Host ("M11-INSTALL-02 installed-app-alive: " + $alive)
    Get-Process aura-shell -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 800
} else {
    Write-Host "M11-INSTALL-02: SKIP（安装目录缺失）"
}

# ---- 04 用户数据保留准备（先造 marker，卸载后验证仍在）----
$appData = Join-Path $env:APPDATA "com.ifuyo.aura"
New-Item -ItemType Directory -Force -Path $appData | Out-Null
Set-Content -Path (Join-Path $appData "m11-smoke-marker.txt") -Value "M11 smoke marker (synthetic)"

# ---- 03 卸载（NSIS uninstaller /S）----
$uninst = Join-Path $installDir "uninstall.exe"
if (Test-Path $uninst) {
    $u = Start-Process $uninst -ArgumentList "/S" -PassThru
    $sw2 = [System.Diagnostics.Stopwatch]::StartNew()
    while (-not $u.HasExited -and $sw2.ElapsedMilliseconds -lt 120000) { Start-Sleep -Milliseconds 250 }
    Start-Sleep -Seconds 2
    Write-Host ("M11-INSTALL-03 uninstall-exitcode: " + $u.ExitCode + " dir-removed: " + (-not (Test-Path $exe)))
} else {
    Write-Host "M11-INSTALL-03: FAIL（未找到 uninstall.exe）"
}

# ---- 04 用户数据保留 ----
Write-Host ("M11-INSTALL-04 user-data-preserved: " + (Test-Path (Join-Path $appData "m11-smoke-marker.txt")))
Remove-Item (Join-Path $appData "m11-smoke-marker.txt") -Force -ErrorAction SilentlyContinue

Get-Process aura-shell -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Host "=== installer smoke done ==="
