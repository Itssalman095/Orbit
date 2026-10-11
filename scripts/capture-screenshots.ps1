# Captures real screenshots of the running app using the simulation harness:
# opens the radial menu (trigger held), moves selection onto an item, and
# screenshots the overlay region + the settings window.

param(
    [string]$Exe = (Join-Path $PSScriptRoot "..\src-tauri\target\release\orbit.exe"),
    [string]$OutDir = "$env:TEMP\orbit-shots"
)

$ErrorActionPreference = "Continue"
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$logFile = Join-Path $env:APPDATA "com.orbit.launcher\orbit.log"
New-Item -ItemType Directory -Force $OutDir | Out-Null

Get-Process -Name orbit -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400
Remove-Item $logFile -ErrorAction SilentlyContinue

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Exe
$psi.Arguments = "--hidden"
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.EnvironmentVariables["ORBIT_SIMULATE"] = "1"
$proc = [System.Diagnostics.Process]::Start($psi)

# wait for readiness
$deadline = (Get-Date).AddSeconds(15)
while ((Get-Date) -lt $deadline) {
    if ((Test-Path $logFile) -and (Select-String -Path $logFile -Pattern "overlay webview ready" -Quiet)) { break }
    Start-Sleep -Milliseconds 150
}

function Shoot([int]$x, [int]$y, [int]$w, [int]$h, [string]$file) {
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($x, $y, 0, 0, (New-Object System.Drawing.Size($w, $h)))
    $g.Dispose()
    $bmp.Save($file, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Output "saved $file"
}

$cx = 700; $cy = 520
$proc.StandardInput.WriteLine(('{{"kind":"down","x":{0},"y":{1}}}' -f $cx, $cy))
Start-Sleep -Milliseconds 900   # open animation completes

# parse geometry from log
$line = (Select-String -Path $logFile -Pattern "menu open: center=\((\d+),(\d+)\) size=(\d+)x(\d+)" | Select-Object -Last 1).Line
$m = [regex]::Match($line, "menu open: center=\((\d+),(\d+)\) size=(\d+)x(\d+)")
$mcx = [int]$m.Groups[1].Value; $mcy = [int]$m.Groups[2].Value
$mw = [int]$m.Groups[3].Value; $mh = [int]$m.Groups[4].Value

# 1. Menu open, cursor at center (dead zone, no selection)
Shoot ($mcx - $mw/2) ($mcy - $mh/2) $mw $mh (Join-Path $OutDir "menu-open.png")

# 2. Selection on item 0 (up)
$proc.StandardInput.WriteLine(('{{"kind":"move","x":{0},"y":{1}}}' -f $mcx, ($mcy - 130)))
Start-Sleep -Milliseconds 500
Shoot ($mcx - $mw/2) ($mcy - $mh/2) $mw $mh (Join-Path $OutDir "menu-selected.png")

# 3. Selection on item 2 (down-left) to see the ring move
$proc.StandardInput.WriteLine(('{{"kind":"move","x":{0},"y":{1}}}' -f ($mcx - 90), ($mcy + 90)))
Start-Sleep -Milliseconds 500
Shoot ($mcx - $mw/2) ($mcy - $mh/2) $mw $mh (Join-Path $OutDir "menu-selected2.png")

# 4. Settings window (show it via tray path: launch a second instance -> shows settings)
Shoot 0 0 [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Height (Join-Path $OutDir "desktop-full.png")

# release and quit
$proc.StandardInput.WriteLine(('{{"kind":"up","x":{0},"y":{1}}}' -f ($mcx - 90), ($mcy + 90)))
Start-Sleep -Milliseconds 800
Get-Process -Name notepad -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Process -Name orbit -ErrorAction SilentlyContinue | Stop-Process -Force
Write-Output "done"
