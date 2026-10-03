# M6 §308：从当前真实构建截取界面截图（非 marketing render）。
# 前置：cd ui && npm run build（ui/dist）+ cargo build -p aura-shell。
# 用法：powershell -ExecutionPolicy Bypass -File scripts/capture-screens.ps1 [-OutDir docs/screenshots]
# 机制：全部导航走已实现快捷键（Alt+1/2/3、Ctrl+K、Ctrl+Q，D-078）；
# mini player 经 palette 命令打开（键盘可编程）；CopyFromScreen 抓窗口。
param(
    [string]$OutDir = "docs/screenshots",
    [string]$Exe = ""
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$repo = Split-Path -Parent $PSScriptRoot
if ($Exe -eq "") {
    $Exe = Join-Path $repo "target/debug/aura-shell.exe"
}
if (-not (Test-Path $Exe)) { throw "app exe not found: $Exe (先 cargo build -p aura-shell)" }
if (-not (Test-Path (Join-Path $repo "ui/dist/index.html"))) { throw "ui/dist missing (先 cd ui && npm run build)" }

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Cap {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

$ws = New-Object -ComObject WScript.Shell

function Capture-Window([string]$TitleMatch, [string]$OutFile) {
    Start-Sleep -Milliseconds 1500
    $proc = Get-Process | Where-Object { $_.MainWindowTitle -like "*$TitleMatch*" -and $_.Id -ne $PID } | Select-Object -First 1
    if (-not $proc) { throw "window not found: $TitleMatch" }
    [Win32Cap]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 500
    $r = New-Object Win32Cap+RECT
    [Win32Cap]::GetWindowRect($proc.MainWindowHandle, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { throw "bad window rect for $TitleMatch" }
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
    $bmp.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Host "captured $(Split-Path -Leaf $OutFile) ($w x $h)"
}

function Capture-Small-Window([string]$OutFile) {
    # mini 窗口标题与主窗一致（tauri.conf 覆盖），按尺寸识别（340x110 附近）。
    Start-Sleep -Milliseconds 800
    $cands = Get-Process aura-shell -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 }
    foreach ($p in $cands) {
        $r = New-Object Win32Cap+RECT
        [Win32Cap]::GetWindowRect($p.MainWindowHandle, [ref]$r) | Out-Null
        $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
        if ($w -gt 100 -and $w -lt 700 -and $h -gt 60 -and $h -lt 300) {
            [Win32Cap]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
            Start-Sleep -Milliseconds 400
            $bmp = New-Object System.Drawing.Bitmap($w, $h)
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
            $bmp.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
            $g.Dispose(); $bmp.Dispose()
            Write-Host "captured $(Split-Path -Leaf $OutFile) ($w x $h)"
            return
        }
    }
    Write-Host "mini window not found (skip 04)"
}

function Focus-Main([object]$Proc) {
    $ws.AppActivate($Proc.Id) | Out-Null
    Start-Sleep -Milliseconds 300
}

# --- 主窗口流程 ---
$proc = Start-Process -FilePath $Exe -PassThru
try {
    Capture-Window "ifuyo Aura" (Join-Path $OutDir "01-now-playing.png")

    Focus-Main $proc; $ws.SendKeys("%2")            # Alt+2 → Library
    Capture-Window "ifuyo Aura" (Join-Path $OutDir "02-library.png")

    Focus-Main $proc; $ws.SendKeys("^k")            # Ctrl+K → Palette
    Capture-Window "ifuyo Aura" (Join-Path $OutDir "03-command-palette.png")
    Focus-Main $proc; $ws.SendKeys("{ESC}")

    Focus-Main $proc; $ws.SendKeys("^q")            # Ctrl+Q → Queue
    Capture-Window "ifuyo Aura" (Join-Path $OutDir "07-queue.png")
    Focus-Main $proc; $ws.SendKeys("^q")

    Focus-Main $proc; $ws.SendKeys("%3")            # Alt+3 → Map
    Capture-Window "ifuyo Aura" (Join-Path $OutDir "05-map-shell.png")

    # Ctrl+M → Mini（palette 输入会被中文 IME 拦截，D-078）。
    Focus-Main $proc; $ws.SendKeys("^m")
    Start-Sleep -Milliseconds 3000
    Capture-Small-Window (Join-Path $OutDir "04-mini-player.png")
}
finally {
    Get-Process | Where-Object { $_.Path -eq $Exe } | Stop-Process -Force -ErrorAction SilentlyContinue
}

# 08 空库 / 09 错误态为「建议」项：01/02 在无库环境下本身即空态与
# 未播放态，可复用；如需专门触发（如 SOURCE_TIMEOUT），
# 在 palette 输入一个不可达 URL 后回车，再截 09-error-state.png。
Write-Host "done -> $OutDir"
