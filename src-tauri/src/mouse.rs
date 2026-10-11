//! Global mouse hook (`WH_MOUSE_LL`) — the heart of the launcher.
//!
//! Design:
//! - A dedicated thread owns the hook and its message pump (LL hooks require
//!   a pumping thread or Windows silently removes them).
//! - The hook callback classifies mouse messages, drives the menu state
//!   machine, computes selection natively, and *consumes* the trigger
//!   button's events while the launcher owns them, so the underlying
//!   application never sees stray presses.
//! - Threading rules enforced here:
//!     * the menu mutex is held only to gather decisions/payloads;
//!     * Tauri events are emitted after the lock is dropped (Rust listeners
//!       run synchronously on the emitting thread);
//!     * window operations are scheduled via `run_on_main_thread`
//!       (fire-and-forget), never performed on the hook thread.
//! - The overlay is click-through; this hook is the single owner of all
//!   pointer routing while a menu is visible.
//!
//! Limitations (documented): a low-level hook callback has a system timeout
//! (`LowLevelHooksTimeout`, ~300 ms default); our callback is O(µs) so this
//! is not an issue in practice. If a heavy third-party hook stalls input
//! delivery, Windows may drop the slowest hook — restarting RRRex re-arms it.

use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, HHOOK, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_XBUTTONDOWN,
    WM_XBUTTONUP,
};

use crate::config::{Config, Settings, Theme, TriggerButton};
use crate::menu::{
    decide_release, decide_sticky_click, ClickOutcome, Geometry, MenuAppDto, MenuRuntime,
    PendingShow,
};
use crate::selection;
use crate::state::{self, SharedState};
use crate::win32;

// ---------------------------------------------------------------------------
// Hook installation / teardown
// ---------------------------------------------------------------------------

struct HookContext {
    app: AppHandle,
}

static CTX: OnceLock<HookContext> = OnceLock::new();
static HOOK_PTR: AtomicIsize = AtomicIsize::new(0);
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
static FIRST_EVENT: AtomicU32 = AtomicU32::new(0);

const WM_QUIT: u32 = 0x0012;

/// Installs the global hook (idempotent). Returns false on failure.
pub fn install(app: AppHandle) -> bool {
    if HOOK_THREAD_ID.load(Ordering::SeqCst) != 0 {
        return true; // already installed
    }
    let _ = CTX.set(HookContext { app: app.clone() });

    let (tx, rx) = mpsc::channel::<bool>();
    let spawned = std::thread::Builder::new()
        .name("rrrex-mouse-hook".into())
        .spawn(move || hook_thread(tx));
    if spawned.is_err() {
        crate::logger::error("failed to spawn mouse hook thread");
        return false;
    }
    match rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(true) => {
            crate::logger::info("mouse hook installed");
            true
        }
        _ => {
            crate::logger::error("mouse hook installation failed or timed out");
            false
        }
    }
}

/// Uninstalls the hook; safe to call from any thread.
pub fn uninstall() {
    let tid = HOOK_THREAD_ID.swap(0, Ordering::SeqCst);
    if tid != 0 {
        unsafe {
            // Ask the hook thread to pump a WM_QUIT and unhook itself.
            let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        crate::logger::info("mouse hook uninstall requested");
    }
}

// ---------------------------------------------------------------------------
// Simulation harness (debug/test only)
// ---------------------------------------------------------------------------

/// When RRREX_SIMULATE=1 is set, a background thread reads JSON lines from
/// stdin and feeds them through the same input state machine as the real
/// hook: {"kind":"down"|"up"|"move"|"ldown","x":960,"y":540}.
///
/// This exists because the development workstation used for automated
/// verification silently drops SendInput (verified with known-good C# and
/// Rust WH_MOUSE_LL probes), which makes OS-level input injection unusable
/// there. The simulator validates the entire pipeline — menu open, overlay
/// show, selection tracking, release-to-launch and cleanup — against the
/// real windows and real process launching. It is inert unless the env var
/// is set and stdin is attached.
pub fn start_simulation_listener() {
    if std::env::var("RRREX_SIMULATE").is_err() {
        return;
    }
    crate::logger::warn("SIMULATION MODE enabled (RRREX_SIMULATE=1): reading events from stdin");
    std::thread::Builder::new()
        .name("rrrex-sim".into())
        .spawn(|| {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let v: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or_default();
                let x = v.get("x").and_then(|k| k.as_i64()).unwrap_or(0) as i32;
                let y = v.get("y").and_then(|k| k.as_i64()).unwrap_or(0) as i32;
                match kind {
                    "list-apps" => {
                        let apps = crate::scanner::scan_installed_apps();
                        crate::logger::info(&format!(
                            "sim: catalog has {} apps; first 5: [{}]",
                            apps.len(),
                            apps.iter()
                                .take(5)
                                .map(|a| a.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                        let _ = (x, y);
                        continue;
                    }
                    "down" => {
                        handle_trigger((x, y), true, TriggerButton::Middle);
                    }
                    "up" => {
                        handle_trigger((x, y), false, TriggerButton::Middle);
                    }
                    "move" => {
                        handle_move((x, y));
                    }
                    "ldown" => {
                        handle_other_button((x, y), true);
                    }
                    _ => {}
                }
                crate::logger::info(&format!("sim event: kind={kind} at ({x},{y})"));
                let _ = v;
            }
        })
        .ok();
}

pub fn is_installed() -> bool {
    HOOK_THREAD_ID.load(Ordering::SeqCst) != 0
}

fn hook_thread(ready: mpsc::Sender<bool>) {
    unsafe {
        // WH_MOUSE_LL is installed with the current module handle.
        let hmod = GetModuleHandleW(None).unwrap_or_default();
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hook_proc), Some(HINSTANCE(hmod.0)), 0);
        match hook {
            Ok(h) => HOOK_PTR.store(h.0 as isize, Ordering::SeqCst),
            Err(e) => {
                crate::logger::error(&format!("SetWindowsHookExW failed: {e}"));
                let _ = ready.send(false);
                return;
            }
        }

        let tid = GetCurrentThreadId();
        HOOK_THREAD_ID.store(tid, Ordering::SeqCst);
        let _ = ready.send(true);

        // Pump messages: required so Windows delivers hook callbacks to this
        // thread. GetMessageW returns 0 on WM_QUIT, -1 on error.
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = DispatchMessageW(&mut msg);
        }

        let p = HOOK_PTR.swap(0, Ordering::SeqCst);
        if p != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(p as *mut _));
        }
        HOOK_THREAD_ID.store(0, Ordering::SeqCst);
        crate::logger::info("mouse hook removed");
    }
}

// ---------------------------------------------------------------------------
// Hook callback
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let msg = wparam.0 as u32;
    let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
    let pt = (info.pt.x, info.pt.y);

    // One-shot diagnostic: proves the hook delivers events at all.
    if FIRST_EVENT.swap(1, Ordering::SeqCst) == 0 {
        crate::logger::info(&format!(
            "first hook event: msg=0x{msg:X} at ({},{})",
            pt.0, pt.1
        ));
    }

    // Fast path: only react to button events and (while open) moves.
    let consumed = match classify(msg, wparam) {
        Some((Button::Middle, down)) => handle_trigger(pt, down, TriggerButton::Middle),
        Some((Button::Right, down)) => handle_trigger(pt, down, TriggerButton::Right),
        Some((Button::X1, down)) => handle_trigger(pt, down, TriggerButton::X1),
        Some((Button::X2, down)) => handle_trigger(pt, down, TriggerButton::X2),
        Some((Button::Left, down)) => handle_other_button(pt, down),
        None => {
            if msg == WM_MOUSEMOVE {
                handle_move(pt)
            } else {
                false
            }
        }
    };

    if consumed {
        LRESULT(1)
    } else {
        CallNextHookEx(None, code, wparam, lparam)
    }
}

/// Returns Some((button, is_down)) for button messages, None otherwise.
fn classify(msg: u32, wparam: WPARAM) -> Option<(Button, bool)> {
    match msg {
        WM_LBUTTONDOWN => Some((Button::Left, true)),
        WM_LBUTTONUP => Some((Button::Left, false)),
        WM_RBUTTONDOWN => Some((Button::Right, true)),
        WM_RBUTTONUP => Some((Button::Right, false)),
        WM_MBUTTONDOWN => Some((Button::Middle, true)),
        WM_MBUTTONUP => Some((Button::Middle, false)),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            let x = ((wparam.0 >> 16) & 0xFFFF) as u16;
            let button = if x == 1 { Button::X1 } else { Button::X2 };
            Some((button, msg == WM_XBUTTONDOWN))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Trigger handling
// ---------------------------------------------------------------------------

/// Result of a trigger down: everything needed to act *outside* the lock.
struct OpenOutcome {
    show: crate::menu::MenuShowDto,
    pending: PendingShow,
    center: (i32, i32),
    scale: f64,
}

fn handle_trigger(pt: (i32, i32), down: bool, button: TriggerButton) -> bool {
    let st = state::get();
    let cfg = st.config_snapshot();

    // Not the configured trigger: route like any other button (sticky clicks).
    if cfg.settings.trigger != button {
        return handle_other_button(pt, down);
    }

    let Some(ctx) = CTX.get() else { return false };
    let app = &ctx.app;

    if down {
        let open = open_menu(&st, &cfg, pt);
        let Some(open) = open else { return false };

        crate::logger::info(&format!(
            "menu open: center=({},{}) size={}x{} apps={} scale={:.2}",
            open.center.0,
            open.center.1,
            open.pending.w,
            open.pending.h,
            open.show.apps.len(),
            open.scale,
        ));

        // 1. Send the render payload (webview-only listeners; async IPC).
        let _ = app.emit_to("overlay", "menu://show", &open.show);

        // 2. Fallback: if the webview never became ready (app just started),
        //    reveal the (transparent) window right away so a slow first load
        //    can't hide the menu forever. The rendered-ack listener on the
        //    main thread handles the normal path.
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let st = state::get();
            if !st.overlay_ready.load(Ordering::SeqCst) {
                let p = take_pending_show(&st);
                if let Some(p) = p {
                    crate::overlay::show_at(&handle, p.x, p.y, p.w, p.h);
                }
            }
        });

        true // consume: the trigger press belongs to the launcher
    } else {
        release_trigger(&st, &cfg, pt, app)
    }
}

/// Gathers the open-menu state under a short lock.
fn open_menu(
    st: &std::sync::Arc<SharedState>,
    cfg: &Config,
    pt: (i32, i32),
) -> Option<OpenOutcome> {
    let geometry = geometry_from_settings(&cfg.settings, win32::dpi_scale_at(pt));
    let items: Vec<crate::menu::MenuItem> = cfg
        .applications
        .iter()
        .filter(|a| a.enabled)
        .map(|a| crate::menu::MenuItem {
            id: a.id.clone(),
            name: a.name.clone(),
        })
        .collect();

    let work = win32::work_area_at(pt);
    let (pending, apps_dto, center, scale) = {
        let mut menu = lock_menu(st);
        let pending = menu.open(pt, work, geometry, items);
        let apps: Vec<MenuAppDto> = menu
            .items
            .iter()
            .map(|item| {
                let cfg_app = cfg.applications.iter().find(|a| a.id == item.id);
                let source = cfg_app.and_then(|a| {
                    a.icon_path
                        .clone()
                        .or_else(|| Some(a.executable_path.clone()))
                });
                let icon = source.and_then(|s| st.icon_for(&s));
                MenuAppDto {
                    id: item.id.clone(),
                    name: item.name.clone(),
                    icon,
                }
            })
            .collect();
        (pending, apps, menu.center, menu.geometry.scale)
    }; // menu lock released before any emission

    let show = crate::menu::MenuShowDto {
        empty_hint: if apps_dto.is_empty() {
            Some("No apps configured\nOpen RRRex settings to add apps".to_string())
        } else {
            None
        },
        apps: apps_dto,
        radius: cfg.settings.radius,
        icon_size: cfg.settings.icon_size,
        start_angle_deg: cfg.settings.start_angle_deg,
        animation_speed: cfg.settings.animation_speed,
        theme: theme_name(&cfg.settings.theme),
        menu_opacity: cfg.settings.menu_opacity,
        selection_glow: cfg.settings.selection_glow,
    };

    Some(OpenOutcome {
        show,
        pending,
        center,
        scale,
    })
}

/// Handles the trigger release. Returns whether the event is consumed.
fn release_trigger(
    st: &std::sync::Arc<SharedState>,
    cfg: &Config,
    _pt: (i32, i32),
    app: &AppHandle,
) -> bool {
    // Gather under the lock.
    enum Resolution {
        Launch(String),
        Sticky,
        Close,
        Idle,
    }

    let resolution = {
        let mut menu = lock_menu(st);
        match menu.phase {
            crate::menu::Phase::Open => {
                let decision = decide_release(
                    menu.selection,
                    cfg.settings.launch_on_release,
                    cfg.settings.sticky_when_no_selection,
                );
                if let Some(index) = decision.launch {
                    let id = menu.items.get(index).map(|i| i.id.clone());
                    match id {
                        Some(id) => {
                            menu.close();
                            Resolution::Launch(id)
                        }
                        None => {
                            menu.close();
                            Resolution::Close
                        }
                    }
                } else if decision.sticky {
                    menu.go_sticky();
                    Resolution::Sticky
                } else {
                    menu.close();
                    Resolution::Close
                }
            }
            crate::menu::Phase::Sticky => Resolution::Idle,
            crate::menu::Phase::Idle => Resolution::Idle,
        }
    }; // lock released

    match resolution {
        Resolution::Launch(id) => {
            let target = cfg.applications.iter().find(|a| a.id == id).cloned();
            let name = target.as_ref().map(|t| t.name.clone()).unwrap_or_default();
            if let Some(target) = target {
                let handle = app.clone();
                std::thread::spawn(move || {
                    crate::launcher::launch_and_report(&handle, &target);
                });
            }
            crate::logger::info(&format!("launch requested: {name}"));
            close_menu(app);
            true
        }
        Resolution::Sticky => {
            crate::logger::info("menu entered sticky mode");
            schedule_click_through(app, false);
            true
        }
        Resolution::Close => {
            close_menu(app);
            true
        }
        // Stray release (down happened before the launcher was ready):
        // consume it so no other app acts on half of our gesture.
        Resolution::Idle => true,
    }
}

/// Hides the menu and resets overlay input, scheduled on the main thread.
fn close_menu(app: &AppHandle) {
    let _ = app.emit_to("overlay", "menu://hide", ());
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        crate::overlay::hide(&handle);
        crate::overlay::set_click_through(&handle, true);
        crate::logger::info("menu closed");
    });
}

fn schedule_click_through(app: &AppHandle, enabled: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        crate::overlay::set_click_through(&handle, enabled);
    });
}

fn take_pending_show(st: &std::sync::Arc<SharedState>) -> Option<PendingShow> {
    let mut menu = lock_menu(st);
    menu.pending_show.take()
}

fn lock_menu(st: &std::sync::Arc<SharedState>) -> std::sync::MutexGuard<'_, MenuRuntime> {
    match st.menu.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Movement + non-trigger buttons
// ---------------------------------------------------------------------------

fn handle_move(pt: (i32, i32)) -> bool {
    let st = state::get();
    let now = now_ms();

    let payload = {
        let mut menu = lock_menu(&st);
        match menu.phase {
            crate::menu::Phase::Open => menu.update_selection(pt, now),
            crate::menu::Phase::Sticky => menu.sticky_cursor_update(pt, now),
            crate::menu::Phase::Idle => None,
        }
    }; // lock released before emission

    if let Some(payload) = payload {
        if let Some(ctx) = CTX.get() {
            let _ = ctx.app.emit_to("overlay", "menu://selection", &payload);
        }
    }

    // Moves always pass through; the overlay is click-through and the
    // underlying window keeps receiving its own mouse-move stream.
    false
}

fn handle_other_button(pt: (i32, i32), down: bool) -> bool {
    let st = state::get();

    enum ClickResolution {
        Launch(String),
        Dismiss,
        DismissAndPass,
    }

    let resolution = {
        let menu = lock_menu(&st);
        if !down || menu.phase != crate::menu::Phase::Sticky {
            return false;
        }
        match decide_sticky_click(&menu, pt.0 as f64, pt.1 as f64) {
            ClickOutcome::Launch(index) => {
                let id = menu.items.get(index).map(|i| i.id.clone());
                match id {
                    Some(id) => ClickResolution::Launch(id),
                    None => ClickResolution::Dismiss,
                }
            }
            ClickOutcome::Dismiss => ClickResolution::Dismiss,
            ClickOutcome::DismissAndPassThrough => ClickResolution::DismissAndPass,
        }
    }; // lock released
    {
        // Clear the menu before acting.
        let mut menu = lock_menu(&st);
        menu.close();
    }

    let Some(ctx) = CTX.get() else { return false };
    let app = &ctx.app;

    match resolution {
        ClickResolution::Launch(id) => {
            let cfg = st.config_snapshot();
            let target = cfg.applications.iter().find(|a| a.id == id).cloned();
            let name = target.as_ref().map(|t| t.name.clone()).unwrap_or_default();
            close_menu(app);
            if let Some(target) = target {
                let handle = app.clone();
                std::thread::spawn(move || {
                    crate::launcher::launch_and_report(&handle, &target);
                });
            }
            crate::logger::info(&format!("sticky launch: {name}"));
            true
        }
        ClickResolution::Dismiss => {
            close_menu(app);
            true
        }
        // Let the user's click reach the app behind, but close the menu.
        ClickResolution::DismissAndPass => {
            close_menu(app);
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn geometry_from_settings(s: &Settings, scale: f64) -> Geometry {
    Geometry {
        radius: s.radius,
        icon_size: s.icon_size,
        dead_zone: s.dead_zone_radius,
        start_angle_rad: selection::deg_to_rad(s.start_angle_deg),
        hysteresis_rad: selection::deg_to_rad(s.hysteresis_deg),
        scale,
    }
}

fn theme_name(t: &Theme) -> String {
    match t {
        Theme::Dark => "dark".to_string(),
        Theme::Light => "light".to_string(),
    }
}
