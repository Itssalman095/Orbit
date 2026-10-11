//! Thin Win32 wrappers: DPI awareness, cursor position, monitor work areas,
//! DPI scale and non-activating window helpers. All coordinates are physical
//! pixels (the process is Per-Monitor-V2 aware).

#![allow(non_snake_case)]

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, GWL_EXSTYLE, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use crate::geometry::WorkArea;

/// Must be called before any window is created so that all coordinates
/// (hooks, GetCursorPos, monitor rects, window positions) are physical.
pub fn init_dpi_awareness() {
    unsafe {
        // May fail if already set by the runtime — that is fine.
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Work area of the monitor containing the point (nearest monitor fallback).
pub fn work_area_at(pt: (i32, i32)) -> WorkArea {
    unsafe {
        let mon: HMONITOR = MonitorFromPoint(POINT { x: pt.0, y: pt.1 }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(mon, &mut mi).as_bool() {
            WorkArea {
                left: mi.rcWork.left,
                top: mi.rcWork.top,
                right: mi.rcWork.right,
                bottom: mi.rcWork.bottom,
            }
        } else {
            // Worst-case fallback: primary-screen-ish area.
            WorkArea {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            }
        }
    }
}

/// DPI scale factor (physical px per logical px) at a point.
pub fn dpi_scale_at(pt: (i32, i32)) -> f64 {
    unsafe {
        let mon = MonitorFromPoint(POINT { x: pt.0, y: pt.1 }, MONITOR_DEFAULTTONEAREST);
        let mut dx: u32 = 96;
        let mut dy: u32 = 96;
        if GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() && dx > 0 {
            dx as f64 / 96.0
        } else {
            1.0
        }
    }
}

/// Extends the overlay window with WS_EX_NOACTIVATE and WS_EX_TOOLWINDOW so
/// it never takes focus and never appears in the taskbar/alt-tab.
pub fn add_no_activate_styles(hwnd: HWND) {
    unsafe {
        let style = windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = style | WS_EX_NOACTIVATE.0 as isize | WS_EX_TOOLWINDOW.0 as isize;
        let _ =
            windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
    }
}

/// Positions, sizes and shows a window without activating it.
pub fn show_window_no_activate(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> bool {
    let flags: SET_WINDOW_POS_FLAGS = SWP_NOACTIVATE | SWP_SHOWWINDOW;
    unsafe { SetWindowPos(hwnd, None, x, y, w, h, flags).is_ok() }
}
