# M10 下篇原生验证（§268–§277/§280–§281/§355/§384/§386）：
# 真实 Windows 环境驱动 tauri build 产物，输出 M10-* 证据行。
#   M10-SINGLE-01..03  单实例（B 退出/A 存活/进程数 1；§277/§386）
#   M10-SMTC-01        SMTC 会话注册（系统 GSMTC 会话管理器列出本应用；§269）
#   M10-SMTC-02        按钮回环读取 —— BLOCKED：PS5.1 WinRT 投影无法解包
#                      会话属性（工具限制；人工按媒体键目测 = 手动 QA）
#   M10-CRASH-01       强杀后重启自恢复（§281）
#   M10-ZOMBIE-01      WM_CLOSE 正式退出后无残留进程（§355/§361）
# 前置：npx --prefix ui tauri build。
param(
    [string]$Exe = ""
)
$ErrorActionPreference = "Continue"

$repo = Split-Path -Parent $PSScriptRoot
if ($Exe -eq "") { $Exe = Join-Path $repo "target/release/aura-shell.exe" }
if (-not (Test-Path $Exe)) { throw "exe missing: $Exe" }
Get-Process aura-shell -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500

Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager, Windows.Media.Control, ContentType = WindowsRuntime]
$null = [Windows.Media.Control.GlobalSystemMediaTransportControlsSession, Windows.Media.Control, ContentType = WindowsRuntime]
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() |
    Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($WinRtOp, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtOp))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}

Write-Host "=== M10 Native Verification ==="

# ---- M10-SINGLE-01..03（§277/§386）----
$swAll = [System.Diagnostics.Stopwatch]::StartNew()
$a = Start-Process $Exe -PassThru
Start-Sleep -Seconds 6
Write-Host ("M10-SINGLE-01 A-alive: " + (-not $a.HasExited))

$b = Start-Process $Exe -PassThru
$sw = [System.Diagnostics.Stopwatch]::StartNew()
while (-not $b.HasExited -and $sw.ElapsedMilliseconds -lt 15000) { Start-Sleep -Milliseconds 100 }
Write-Host ("M10-SINGLE-02 B-exited: " + $b.HasExited + " (latency_ms=" + $sw.ElapsedMilliseconds + ")")

$count = (Get-Process aura-shell -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host ("M10-SINGLE-03 process-count: " + $count)

# ---- M10-SMTC-01（§269：SMTC 会话注册进系统会话管理器）----
try {
    $manager = Await ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager]::RequestAsync()) ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager])
    Start-Sleep -Seconds 1
    $sessions = $manager.GetSessions()
    $found = $false
    foreach ($s in $sessions) {
        $appid = ""
        try { $appid = $s.SourceAppUserModelId } catch { $appid = "?" }
        Write-Host ("M10-SMTC-01 session: appid='" + $appid + "'")
        if ($appid -like '*aura-shell*') { $found = $true }
    }
    Write-Host ("M10-SMTC-01 session-registered: " + $found)
    Write-Host "M10-SMTC-02 button-roundtrip: BLOCKED (PS5.1 WinRT projection cannot unwrap session properties; manual media-key check = manual QA)"
} catch {
    Write-Host ("M10-SMTC-01: BLOCKED (" + $_.Exception.Message + ")")
}

# ---- M10-CRASH-01（§281：强杀 → 重启自恢复）----
Stop-Process -Name aura-shell -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 800
$c = Start-Process $Exe -PassThru
Start-Sleep -Seconds 5
Write-Host ("M10-CRASH-01 relaunch-after-kill alive: " + (-not $c.HasExited))

# ---- M10-ZOMBIE-01（§355/§361：WM_CLOSE 正式退出 → 无残留进程）----
Add-Type @"
using System; using System.Runtime.InteropServices;
public class ZK { [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string cls, string title); [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l); }
"@
$app = Get-Process aura-shell -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$h = if ($app) { $app.MainWindowHandle } else { [IntPtr]::Zero }
Write-Host ("M10-ZOMBIE-01 main-window-handle: " + ($h -ne [IntPtr]::Zero))
if ($h -ne [IntPtr]::Zero) {
    [ZK]::PostMessageW($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Start-Sleep -Seconds 3
}
$count2 = (Get-Process aura-shell -ErrorAction SilentlyContinue | Measure-Object).Count
Write-Host ("M10-ZOMBIE-01 process-count-after-close: " + $count2)
if ($count2 -eq 0) { Write-Host "M10-ZOMBIE-01 PASS (formal exit, no zombie)" }

Get-Process aura-shell -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Host ("=== done (total " + [int]$swAll.Elapsed.TotalSeconds + "s) ===")
