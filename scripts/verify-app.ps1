# End-to-end verification harness for RRRex on Windows.
#
# Drives the real application with SendInput:
#   1. makes this PowerShell process Per-Monitor-V2 DPI aware (physical px)
#   2. moves the cursor to a safe screen position
#   3. presses the middle mouse button (global trigger)
#   4. reads the menu center + scale from the app log
#   5. moves the cursor onto item 0 (12 o'clock position)
#   6. releases the button -> the app must launch the selected app
#   7. verifies the child process was created
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\verify-app.ps1 [-Exe <path>]

param(
    [string]$Exe = (Join-Path $PSScriptRoot "..\src-tauri\target\release\rrrex.exe"),
    [string]$ChildExe = "notepad.exe",
    [int]$MaxWaitSec = 15
)

$ErrorActionPreference = "Continue"

$childProcName = [System.IO.Path]::GetFileNameWithoutExtension($ChildExe)
$appProcName = [System.IO.Path]::GetFileNameWithoutExtension($Exe)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class NativeInput {
    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    public static extern bool GetCursorPos(out POINT p);
    [DllImport("user32.dll")]
    public static extern int SendInput(int n, INPUT[] pInputs, int cbSize);
    [DllImport("user32.dll")]
    public static extern bool SystemParametersInfo(uint action, uint param, ref RECT rect, uint winini);

    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSEINPUT {
        public int dx; public int dy; public int mouseData;
        public uint dwFlags; public uint time; public IntPtr dwExtraInfo;
    }
    [StructLayout(LayoutKind.Explicit)]
    public struct INPUT {
        [FieldOffset(0)] public uint type;
        [FieldOffset(8)] public MOUSEINPUT mi;
    }

    public const uint MOUSEEVENTF_MIDDLEDOWN = 0x0040;
    public const uint MOUSEEVENTF_MIDDLEUP   = 0x0080;
    public const int INPUT_MOUSE = 0;
}
"@

# DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4
[void][NativeInput]::SetProcessDpiAwarenessContext([IntPtr](-4))

$logFile = Join-Path $env:APPDATA "com.rrrex.launcher\rrrex.log"

function Log-Tail([string]$pattern, [int]$waitSec) {
    $deadline = (Get-Date).AddSeconds($waitSec)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path $logFile) {
            $m = Select-String -Path $logFile -Pattern $pattern | Select-Object -Last 1
            if ($m) { return $m.Line }
        }
        Start-Sleep -Milliseconds 100
    }
    return $null
}

Write-Output "=== RRRex end-to-end verification ==="
Write-Output "exe: $Exe"
Write-Output "log: $logFile"

if (-not (Test-Path $Exe)) { Write-Output "FAIL: exe not found"; exit 1 }

# Kill any previous instance (single-instance plugin would defer otherwise).
Get-Process -Name $appProcName -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300
if (Test-Path $logFile) { Remove-Item $logFile -ErrorAction SilentlyContinue }

# 1. Start the app.
Start-Process -FilePath $Exe
$bootLine = Log-Tail "overlay webview ready" $MaxWaitSec
if (-not $bootLine) {
    Write-Output "FAIL: app did not signal overlay ready"
    Get-Process -Name "rrrex" -ErrorAction SilentlyContinue | Stop-Process -Force
    exit 1
}
Write-Output "[ok] app started, overlay webview ready"

Start-Sleep -Milliseconds 500

# 2. Position the cursor at a safe point: center of the primary work area.
[void][System.Reflection.Assembly]::LoadWithPartialName("System.Windows.Forms")
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$cx = [int]($screen.Width / 2)
$cy = [int]($screen.Height / 2)
[void][NativeInput]::SetCursorPos($cx, $cy)
Start-Sleep -Milliseconds 200

# 3. Middle mouse button DOWN.
$down = New-Object NativeInput+INPUT
$down.type = [NativeInput]::INPUT_MOUSE
$down.mi.dwFlags = [NativeInput]::MOUSEEVENTF_MIDDLEDOWN
[void][NativeInput]::SendInput(1, @($down), [System.Runtime.InteropServices.Marshal]::SizeOf([type][NativeInput+INPUT]))

$openLine = Log-Tail "menu open: center=\((\d+),(\d+)\).*scale=([0-9.]+)" $MaxWaitSec
if (-not $openLine) {
    Write-Output "FAIL: menu did not open on middle-button down"
    $up = New-Object NativeInput+INPUT
    $up.type = [NativeInput]::INPUT_MOUSE
    $up.mi.dwFlags = [NativeInput]::MOUSEEVENTF_MIDDLEUP
    [void][NativeInput]::SendInput(1, @($up), [System.Runtime.InteropServices.Marshal]::SizeOf([type][NativeInput+INPUT]))
    Get-Process -Name "rrrex" -ErrorAction SilentlyContinue | Stop-Process -Force
    exit 1
}
Write-Output "[ok] trigger down detected by the global hook: $openLine"

# Parse center + scale from the log line.
$regex = [regex]"menu open: center=\((\d+),(\d+)\).*scale=([0-9.]+)"
$parsed = $regex.Match($openLine)
$centerX = [int]$parsed.Groups[1].Value
$centerY = [int]$parsed.Groups[2].Value
$scale = [double]$parsed.Groups[3].Value
Write-Output ("     center=({0},{1}) scale={2}" -f $centerX, $centerY, $scale)

# 4. Wait for the overlay window to become visible (rendered-ack path).
$shownLine = Log-Tail "overlay shown" 3
if ($shownLine) { Write-Output "[ok] $shownLine" }
else { Write-Output "[warn] overlay show not confirmed (rendered-ack missing)" }

# 5. Move the cursor onto item 0 (12 o'clock, radius 140 logical px up).
$radiusPhys = [int][Math]::Round(140 * $scale)
$itemX = $centerX
$itemY = $centerY - $radiusPhys
[void][NativeInput]::SetCursorPos($itemX, $itemY)
Start-Sleep -Milliseconds 350   # let the hook process the move + selection

# 6. Middle mouse button UP -> launches the selected app.
$up = New-Object NativeInput+INPUT
$up.type = [NativeInput]::INPUT_MOUSE
$up.mi.dwFlags = [NativeInput]::MOUSEEVENTF_MIDDLEUP
[void][NativeInput]::SendInput(1, @($up), [System.Runtime.InteropServices.Marshal]::SizeOf([type][NativeInput+INPUT]))

$launchLine = Log-Tail "launch requested: (\S.*)|launch '.*' -> ok" $MaxWaitSec
if ($launchLine) { Write-Output "[ok] launch dispatched: $launchLine" }
else { Write-Output "FAIL: no launch after release" }

# 7. Verify the child process actually started.
$child = $null
$deadline = (Get-Date).AddSeconds($MaxWaitSec)
while ((Get-Date) -lt $deadline -and -not $child) {
    Start-Sleep -Milliseconds 250
    $child = Get-Process -Name $childProcName -ErrorAction SilentlyContinue | Select-Object -First 1
}

$passed = $true
if ($launchLine -and $child) {
    Write-Output "[ok] child process '$ChildExe' is running (pid $($child.Id))"
} else {
    $passed = $false
    if (-not $launchLine) { Write-Output "FAIL: launch was not dispatched" }
    if (-not $child) { Write-Output "FAIL: child process '$ChildExe' did not start" }
}

# 8. Check overlay closed after release.
$hideLine = Log-Tail "menu hidden|close" 2
Start-Sleep -Milliseconds 300

# Cleanup: close the child and the app.
if ($child) { Stop-Process -Id $child.Id -Force -ErrorAction SilentlyContinue }
Get-Process -Name $appProcName -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 400
$stillRunning = Get-Process -Name $appProcName -ErrorAction SilentlyContinue
if ($stillRunning) { Write-Output "[warn] app still running after quit" } else { Write-Output "[ok] app terminated" }

Write-Output ""
if ($passed) {
    Write-Output "=== RESULT: PASS ==="
    exit 0
} else {
    Write-Output "=== RESULT: FAIL ==="
    exit 1
}
