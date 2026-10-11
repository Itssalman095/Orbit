# Known-good WH_MOUSE_LL test in C#. Prints every event to the console.
# Used to isolate whether low-level mouse hooks work in this environment.

Add-Type -TypeDefinition @"
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;

public static class CsHook {
    private static IntPtr _hook = IntPtr.Zero;
    private static readonly Stopwatch Sw = Stopwatch.StartNew();

    private delegate IntPtr HookProc(int code, IntPtr wParam, IntPtr lParam);

    [StructLayout(LayoutKind.Sequential)]
    private struct MSLL { public POINT pt; public uint mouseData; public uint flags; public uint time; public IntPtr extra; }

    [StructLayout(LayoutKind.Sequential)]
    private struct POINT { public int X; public int Y; }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetWindowsHookEx(int idHook, HookProc lpfn, IntPtr hMod, uint dwThreadId);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool UnhookWindowsHookEx(IntPtr hhk);

    [DllImport("user32.dll")]
    private static extern IntPtr CallNextHookEx(IntPtr hhk, int code, IntPtr wParam, IntPtr lParam);

    [DllImport("kernel32.dll")]
    private static extern IntPtr GetModuleHandle(string lpModuleName);

    private static HookProc _proc = HookCallback;

    private static IntPtr HookCallback(int code, IntPtr wParam, IntPtr lParam) {
        if (code >= 0) {
            MSLL data = (MSLL)Marshal.PtrToStructure(lParam, typeof(MSLL));
            Console.WriteLine("csharp hook: msg=0x{0:X} at ({1},{2}) t={3}", wParam, data.pt.X, data.pt.Y, Sw.ElapsedMilliseconds);
        }
        return CallNextHookEx(_hook, code, wParam, lParam);
    }

    public static string Install() {
        _hook = SetWindowsHookEx(14, _proc, GetModuleHandle(null), 0);
        if (_hook == IntPtr.Zero) return "install failed: " + Marshal.GetLastWin32Error();
        return "installed";
    }

    public static void Remove() {
        if (_hook != IntPtr.Zero) UnhookWindowsHookEx(_hook);
    }
}
"@

Write-Output ("install: " + [CsHook]::Install())
Write-Output "hook active for 6 seconds - move the mouse / injected input follows..."
Start-Sleep -Seconds 2

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class CsSend {
    [DllImport("user32.dll", SetLastError=true)]
    public static extern int SendInput(int n, INPUT[] pInputs, int cbSize);
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSEINPUT { public int dx; public int dy; public int mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)]
    public struct INPUT { [FieldOffset(0)] public uint type; [FieldOffset(8)] public MOUSEINPUT mi; }
}
"@
[void][CsSend]::SetCursorPos(700, 500)
Start-Sleep -Milliseconds 200
[void][CsSend]::SetCursorPos(800, 500)
$s = [System.Runtime.InteropServices.Marshal]::SizeOf([type][CsSend+INPUT])
$d = New-Object CsSend+INPUT; $d.type = 0; $d.mi.dwFlags = 0x0040
[void][CsSend]::SendInput(1, @($d), $s)
Start-Sleep -Milliseconds 300
$u = New-Object CsSend+INPUT; $u.type = 0; $u.mi.dwFlags = 0x0080
[void][CsSend]::SendInput(1, @($u), $s)

Start-Sleep -Seconds 4
[CsHook]::Remove()
Write-Output "done"
