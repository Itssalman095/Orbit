# Diagnoses which desktop currently owns input. Low-level hooks only see
# events on their own desktop; when a workstation is locked (or an RDP
# session is disconnected), input goes to the secure Winlogon desktop.

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class DesktopDiag {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr OpenInputDesktop(uint flags, bool inherit, uint access);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr OpenDesktop(string name, uint flags, bool inherit, uint access);

    [DllImport("user32.dll")]
    public static extern bool CloseDesktop(IntPtr h);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool GetUserObjectInformation(IntPtr hObj, int nIndex, StringBuilder pvInfo, uint nLength, out uint lpnLengthNeeded);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern bool GetLastInputInfo(ref LASTINPUTINFO plii);

    [StructLayout(LayoutKind.Sequential)]
    public struct LASTINPUTINFO { public uint cbSize; public uint dwTime; }

    public static string DesktopName(IntPtr h) {
        var sb = new StringBuilder(256);
        uint needed;
        if (GetUserObjectInformation(h, 2, sb, 256, out needed)) return sb.ToString();
        return "?";
    }
}
"@

$UOI_NAME = 2
$DESKTOP_READOBJECTS = 0x0001

$defaultH = [DesktopDiag]::OpenDesktop("Default", 0, $false, $DESKTOP_READOBJECTS)
$inputH = [DesktopDiag]::OpenInputDesktop(0, $false, $DESKTOP_READOBJECTS)

Write-Output ("default desktop handle: " + $defaultH)
Write-Output ("input desktop handle:   " + $inputH)
if ($defaultH -ne [IntPtr]::Zero) { Write-Output ("default desktop name:    " + [DesktopDiag]::DesktopName($defaultH)) }
if ($inputH -ne [IntPtr]::Zero) {
    Write-Output ("input desktop name:      " + [DesktopDiag]::DesktopName($inputH))
    [void][DesktopDiag]::CloseDesktop($inputH)
} else {
    Write-Output "input desktop: UNOPENABLE -> session locked or no interactive input"
}
if ($defaultH -ne [IntPtr]::Zero) { [void][DesktopDiag]::CloseDesktop($defaultH) }

$fg = [DesktopDiag]::GetForegroundWindow()
Write-Output ("foreground window: " + $fg)

$li = New-Object DesktopDiag+LASTINPUTINFO
$li.cbSize = [System.Runtime.InteropServices.Marshal]::SizeOf([type][DesktopDiag+LASTINPUTINFO])
[void][DesktopDiag]::GetLastInputInfo([ref]$li)
$uptime = [Environment]::TickCount
Write-Output ("last input tick: " + $li.dwTime + " (age ms: " + ($uptime - $li.dwTime) + ")")
