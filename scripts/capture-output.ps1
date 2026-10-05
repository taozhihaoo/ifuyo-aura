# M12 诊断：黑盒录制系统实际输出（WASAPI loopback）。
# 用法：powershell -File scripts/capture-output.ps1 -Seconds 30 -Out target\sysout.wav
param(
    [int]$Seconds = 30,
    [string]$Out = "target\sysout.wav"
)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$outPath = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $repo $Out }

$dev = ffmpeg -list_devices 2>&1 | Select-String "DirectShow|dshow" | Out-Null
# ffmpeg WASAPI loopback：直接以渲染端点名作为输入
$device = (Get-CimInstance Win32_SoundDevice | Where-Object Name -like "*Realtek*").Name | Select-Object -First 1
# 用默认渲染端点的友好名（扬声器）
$endpoint = (Get-PnpDevice -Class AudioEndpoint -Status OK | Where-Object FriendlyName -like "扬声器*" | Select-Object -First 1).FriendlyName
Write-Host "loopback endpoint: $endpoint"

ffmpeg -hide_banner -loglevel error `
  -f wasapi -i audio="$endpoint" `
  -t $Seconds `
  -acodec pcm_s16le -ar 48000 -ac 2 `
  -y $outPath
Write-Host "captured: $outPath"
