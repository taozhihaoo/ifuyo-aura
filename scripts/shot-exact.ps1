# M13.2：截取标题恰为 "ifuyo Aura" 的窗口（PrintWindow flags=2 支持被遮挡窗口）。
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;using System.Runtime.InteropServices;using System.Text;
public class WA {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumC c, IntPtr l);
  public delegate bool EnumC(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public struct R { public int L, T, Rt, B; }
}
'@
$found = [IntPtr]::Zero
$cb = { param($h, $l)
  if (-not [WA]::IsWindowVisible($h)) { return $true }
  $sb = New-Object System.Text.StringBuilder 256
  [void][WA]::GetWindowText($h, $sb, 256)
  if ($sb.ToString() -eq 'ifuyo Aura') { $script:found = $h; return $false }
  return $true }
[void][WA]::EnumWindows($cb, [IntPtr]::Zero)
if ($found -eq [IntPtr]::Zero) { Write-Output 'NO WINDOW'; exit 1 }
$r = New-Object WA+R
[void][WA]::GetWindowRect($found, [ref]$r)
$bmp = New-Object System.Drawing.Bitmap([int]($r.Rt - $r.L), [int]($r.B - $r.T))
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
[void][WA]::PrintWindow($found, $dc, 2)
$g.ReleaseHdc($dc)
$bmp.Save('E:\WorkSpace\ifuyo-aura\target\ui-round2b.png')
Write-Output 'SAVED'
