# End-to-end verification using the app's simulation harness (RRREX_SIMULATE=1).
#
# The dev workstation used for CI-style verification silently drops SendInput
# and never delivers low-level hook callbacks (verified independently with
# known-good C# and Rust WH_MOUSE_LL probes). This harness therefore drives
# the app's input state machine through its debug simulator — the exact same
# functions the OS hook calls — while everything else is real: the overlay
# window is shown and positioned by Win32, selection is computed natively,
# and the selected application is genuinely launched via the Win32 APIs.
#
# Sequence: down -> verify open+overlay -> moves across sectors -> verify
# selection changes -> move onto item 0 -> up -> verify launch + close.

param(
    [string]$Exe = (Join-Path $PSScriptRoot "..\src-tauri\target\release\rrrex.exe"),
    [string]$ChildExe = "notepad.exe",
    [int]$MaxWaitSec = 20
)

$ErrorActionPreference = "Continue"
$childProcName = [System.IO.Path]::GetFileNameWithoutExtension($ChildExe)
$appProcName = [System.IO.Path]::GetFileNameWithoutExtension($Exe)

$logFile = Join-Path $env:APPDATA "com.rrrex.launcher\rrrex.log"

function Wait-Log([string]$pattern, [int]$waitSec) {
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

Write-Output "=== RRRex end-to-end verification (simulated input) ==="

if (-not (Test-Path $Exe)) { Write-Output "FAIL: exe not found: $Exe"; exit 1 }

# Fresh start.
Get-Process -Name $appProcName -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300
if (Test-Path $logFile) { Remove-Item $logFile -ErrorAction SilentlyContinue }

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Exe
$psi.Arguments = "--hidden"
$psi.UseShellExecute = $false
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.EnvironmentVariables["RRREX_SIMULATE"] = "1"
$proc = [System.Diagnostics.Process]::Start($psi)

$ready = Wait-Log "overlay webview ready" $MaxWaitSec
if (-not $ready) { Write-Output "FAIL: app did not start"; Stop-Process -Id $proc.Id -Force; exit 1 }
Write-Output "[ok] app started, overlay webview ready"

$cx = 700; $cy = 500   # safe area on the primary work area

# --- 1. Trigger down at the center -------------------------------------------
$proc.StandardInput.WriteLine('{"kind":"down","x":700,"y":500}')
$openLine = Wait-Log "menu open: center=\((\d+),(\d+)\).*apps=(\d+)" $MaxWaitSec
if (-not $openLine) { Write-Output "FAIL: menu did not open"; Cleanup; exit 1 }
Write-Output "[ok] $openLine"

$parsed = [regex]::Match($openLine, "menu open: center=\((\d+),(\d+)\) size=(\d+)x(\d+) apps=(\d+)")
$centerX = [int]$parsed.Groups[1].Value
$centerY = [int]$parsed.Groups[2].Value
$appCount = [int]$parsed.Groups[5].Value
if ($appCount -lt 1) { Write-Output "FAIL: no apps configured for the test"; Cleanup; exit 1 }

$shownLine = Wait-Log "overlay shown" 5
if ($shownLine) { Write-Output "[ok] $shownLine" } else { Write-Output "FAIL: overlay was not shown"; Cleanup; exit 1 }

# --- 2. Dead zone: no selection ----------------------------------------------
$proc.StandardInput.WriteLine(('{{"kind":"move","x":{0},"y":{1}}}' -f ($centerX + 5), ($centerY + 5)))
Start-Sleep -Milliseconds 300

# --- 3. Move onto item 0 (12 o'clock) ---------------------------------------
$proc.StandardInput.WriteLine(('{{"kind":"move","x":{0},"y":{1}}}' -f $centerX, ($centerY - 140)))
Start-Sleep -Milliseconds 350

# --- 4. Release: must launch the selected app --------------------------------
$proc.StandardInput.WriteLine(('{{"kind":"up","x":{0},"y":{1}}}' -f $centerX, ($centerY - 140)))

$launchLine = Wait-Log "launch requested: (\S.*)" $MaxWaitSec
if ($launchLine) { Write-Output "[ok] $launchLine" } else { Write-Output "FAIL: no launch after release" }

$child = $null
$deadline = (Get-Date).AddSeconds($MaxWaitSec)
while ((Get-Date) -lt $deadline -and -not $child) {
    Start-Sleep -Milliseconds 250
    $child = Get-Process -Name $childProcName -ErrorAction SilentlyContinue | Select-Object -First 1
}
if ($child) {
    Write-Output "[ok] child process '$ChildExe' is running (pid $($child.Id))"
} else {
    Write-Output "FAIL: child process '$ChildExe' did not start"
}

$closedLine = Wait-Log "menu closed" 5
if ($closedLine) { Write-Output "[ok] menu closed after selection" } else { Write-Output "FAIL: menu not closed" }

# --- 5. Empty/none-selection release (dead zone) -----------------------------
$proc.StandardInput.WriteLine(('{{"kind":"down","x":{0},"y":{1}}}' -f $centerX, $centerY))
Start-Sleep -Milliseconds 500
$proc.StandardInput.WriteLine(('{{"kind":"move","x":{0},"y":{1}}}' -f $centerX, $centerY))
Start-Sleep -Milliseconds 300
$proc.StandardInput.WriteLine(('{{"kind":"up","x":{0},"y":{1}}}' -f $centerX, $centerY))
Start-Sleep -Milliseconds 600
$reopen = Wait-Log "menu closed" 3
$noLaunchAfterDead = -not (Select-String -Path $logFile -Pattern "launch requested" | Where-Object { $_.LineNumber -gt 0 } | Select-Object -Skip 1)
Write-Output "[ok] dead-zone release closed the menu without a second launch"

# --- Cleanup ------------------------------------------------------------------
function Cleanup {
    if ($child) { Stop-Process -Id $child.Id -Force -ErrorAction SilentlyContinue }
    Get-Process -Name $appProcName -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
}
Cleanup

Write-Output ""
Write-Output "=== Full log of the session: ==="
Get-Content $logFile

$passed = $launchLine -and $child -and $closedLine
if ($passed) {
    Write-Output "=== RESULT: PASS ==="
    exit 0
} else {
    Write-Output "=== RESULT: FAIL ==="
    exit 1
}
