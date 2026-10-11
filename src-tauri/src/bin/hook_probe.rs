// Minimal WH_MOUSE_LL hook probe: logs every event to stderr.
// Built and run standalone to isolate hook-thread behavior from the app.
// Usage: cargo run --bin hook_probe  (then move the mouse)

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, MSG, MSLLHOOKSTRUCT,
    WH_MOUSE_LL, WM_MOUSEMOVE,
};

static QUIT: AtomicBool = AtomicBool::new(false);
static TID: OnceLock<u32> = OnceLock::new();

unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let msg = wparam.0 as u32;
    let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
    if msg == WM_MOUSEMOVE {
        eprintln!("hook probe: move ({},{})", info.pt.x, info.pt.y);
    } else {
        eprintln!("hook probe: msg=0x{msg:X}");
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn main() {
    unsafe {
        let hmod = GetModuleHandleW(None).unwrap_or_default();
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(proc), Some(HINSTANCE(hmod.0)), 0)
            .expect("hook install failed");
        let tid = GetCurrentThreadId();
        let _ = TID.set(tid);
        eprintln!("hook probe: installed, tid={tid}");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = DispatchMessageW(&mut msg);
            if QUIT.load(Ordering::SeqCst) {
                break;
            }
        }
        eprintln!("hook probe: loop exited");
        let _ = windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(hook);
    }
}
