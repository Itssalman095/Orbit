//! Overlay window control. All functions here must run on the main thread
//! (winit requirement); the mouse hook asks for these via events.

use tauri::{AppHandle, Manager};

use crate::win32;

/// Overlay window configuration applied once at startup:
/// click-through + no-activate + no taskbar/alt-tab.
pub fn setup(app: &AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else {
        crate::logger::error("overlay window missing from config");
        return;
    };

    if let Err(e) = win.set_ignore_cursor_events(true) {
        crate::logger::error(&format!("failed to set click-through: {e}"));
    }

    match win.hwnd() {
        Ok(hwnd) => win32::add_no_activate_styles(hwnd),
        Err(e) => {
            crate::logger::error(&format!("overlay hwnd unavailable: {e}"));
        }
    }
    crate::logger::info("overlay window initialized (click-through, no-activate)");
}

/// Shows the overlay at the given physical position/size without stealing
/// focus. Called after the webview acknowledged it rendered the payload.
pub fn show_at(app: &AppHandle, x: i32, y: i32, w: i32, h: i32) {
    let Some(win) = app.get_webview_window("overlay") else {
        return;
    };
    let Ok(hwnd) = win.hwnd() else { return };
    if win32::show_window_no_activate(hwnd, x, y, w, h) {
        crate::logger::info(&format!("overlay shown at ({x},{y}) {w}x{h}"));
    } else {
        crate::logger::error("SetWindowPos(SHOW) failed for overlay");
    }
}

pub fn hide(app: &AppHandle) {
    let Some(win) = app.get_webview_window("overlay") else {
        return;
    };
    if let Err(e) = win.hide() {
        crate::logger::error(&format!("failed to hide overlay: {e}"));
    }
}

pub fn set_click_through(app: &AppHandle, enabled: bool) {
    let Some(win) = app.get_webview_window("overlay") else {
        return;
    };
    if let Err(e) = win.set_ignore_cursor_events(enabled) {
        crate::logger::error(&format!("set_ignore_cursor_events({enabled}) failed: {e}"));
    }
}
