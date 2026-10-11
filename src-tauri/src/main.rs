//! Orbit — a radial application launcher for Windows.
//!
//! Process layout:
//! - main thread: tauri event loop, window management, tray
//! - hook thread: global WH_MOUSE_LL hook + menu state machine
//! - worker threads: process launching, icon extraction, config saving
//!
//! Coordinate contract: the process is Per-Monitor-V2 DPI aware, so every
//! native coordinate (hook, GetCursorPos, monitor rects, window positions)
//! is in physical pixels. Logical (CSS px) values from the config are
//! multiplied by the monitor scale factor when the menu opens.

// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod geometry;
mod icons;
mod launcher;
mod logger;
mod menu;
mod mouse;
mod overlay;
mod scanner;
mod selection;
mod state;
mod tray;
mod win32;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use tauri::{Listener, Manager, RunEvent, WindowEvent};

/// Must match `bundle.identifier` in tauri.conf.json.
const APP_IDENTIFIER: &str = "com.orbit.launcher";

/// The pre-rename data directory; migrated on first run (see
/// `migrate_rrrex_data`).
const LEGACY_IDENTIFIER: &str = "com.rrrex.launcher";

fn app_config_dir() -> PathBuf {
    // Test-isolation knob: point the app at a scratch config directory
    // (used by scripts/verify-app-simulated.ps1 so harness runs never touch
    // real user data).
    if let Ok(dir) = std::env::var("ORBIT_CONFIG_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    let base = std::env::var("APPDATA").unwrap_or_else(|_| {
        // Extremely defensive: relative dir if the env var is missing.
        ".".to_string()
    });
    PathBuf::from(base).join(APP_IDENTIFIER)
}

/// One-time migration from the pre-rename data folder (RRRex) so settings,
/// applications and cached icons carry over to Orbit.
fn migrate_rrrex_data(config_dir: &std::path::Path) {
    if config_dir.join(config::CONFIG_FILE).exists() {
        return; // already migrated (or new install with fresh config)
    }
    let Some(base) = std::env::var_os("APPDATA") else {
        return;
    };
    let legacy = PathBuf::from(base).join(LEGACY_IDENTIFIER);
    let legacy_cfg = legacy.join(config::CONFIG_FILE);
    if !legacy_cfg.is_file() {
        return;
    }
    let _ = std::fs::create_dir_all(config_dir);
    if std::fs::copy(&legacy_cfg, config_dir.join(config::CONFIG_FILE)).is_ok() {
        crate::logger::info("migrated configuration from the legacy RRRex data folder");
    }
    // Cached icons are cheap to re-extract, but copying avoids the wait.
    let legacy_icons = legacy.join("icons");
    let new_icons = config_dir.join("icons");
    if legacy_icons.is_dir() {
        let _ = std::fs::create_dir_all(&new_icons);
        if let Ok(entries) = std::fs::read_dir(&legacy_icons) {
            for entry in entries.flatten() {
                let target = new_icons.join(entry.file_name());
                if !target.exists() {
                    let _ = std::fs::copy(entry.path(), target);
                }
            }
        }
    }
}

/// Builds the shared state before any webview can issue commands.
///
/// Tauri creates config windows (and starts loading their webviews) before
/// the setup hook runs, so IPC commands can arrive before setup() — the
/// state must therefore exist before the builder is even constructed.
/// The path mirrors tauri's `app_config_dir()` on Windows
/// (%APPDATA%\<identifier>); setup verifies they agree.
fn init_shared_state() -> std::sync::Arc<state::SharedState> {
    let config_dir = app_config_dir();
    let icon_cache_dir = config_dir.join("icons");
    let _ = std::fs::create_dir_all(&config_dir);
    let _ = std::fs::create_dir_all(&icon_cache_dir);

    migrate_rrrex_data(&config_dir);

    let cfg = config::load(&config_dir);
    std::sync::Arc::new(state::SharedState {
        config: std::sync::RwLock::new(cfg),
        menu: std::sync::Mutex::new(menu::MenuRuntime::default()),
        icons: std::sync::Mutex::new(std::collections::HashMap::new()),
        overlay_ready: std::sync::atomic::AtomicBool::new(false),
        tray_enabled_item: std::sync::Mutex::new(None),
        paths: state::Paths {
            config_dir,
            icon_cache_dir,
        },
    })
}

fn main() {
    win32::init_dpi_awareness();

    // Logging + shared state exist before any webview can issue commands
    // (config windows start loading before setup() runs).
    logger::init(app_config_dir().join("orbit.log"));
    let shared = init_shared_state();
    state::init(shared.clone());

    tauri::Builder::default()
        // Must be first: subsequent instances exit here and hand over to us.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let silent = args.iter().any(|a| a == "--hidden");
            if !silent {
                tray::show_settings(app);
                crate::logger::info("second instance: settings window shown");
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .on_page_load(|webview, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                crate::logger::info(&format!(
                    "page load finished: window={} url={}",
                    webview.label(),
                    payload.url()
                ));
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();

            // ---- Paths sanity check ----------------------------------------
            let tauri_config_dir = app
                .path()
                .app_config_dir()
                .expect("app config dir must be resolvable");
            let st = state::get();
            if tauri_config_dir != st.paths.config_dir {
                crate::logger::warn(&format!(
                    "config dir mismatch: tauri={} state={}",
                    tauri_config_dir.display(),
                    st.paths.config_dir.display()
                ));
            }
            logger::info("Orbit starting");

            // ---- Windows ---------------------------------------------------
            overlay::setup(&handle);
            tray::create(&handle)?;

            // ---- Main-thread listeners -------------------------------------
            // Window operations are scheduled by the hook thread via
            // `run_on_main_thread`; these listeners only cover
            // webview-originated messages.
            {
                app.listen("menu://ready", move |_| {
                    state::get().overlay_ready.store(true, Ordering::SeqCst);
                    logger::info("overlay webview ready");
                });
            }
            {
                let h = handle.clone();
                app.listen("menu://rendered", move |_| {
                    // The overlay committed the new payload: reveal it now.
                    let st = state::get();
                    let pending = {
                        let mut menu = match st.menu.lock() {
                            Ok(m) => m,
                            Err(e) => e.into_inner(),
                        };
                        menu.pending_show.take()
                    };
                    if let Some(p) = pending {
                        overlay::show_at(&h, p.x, p.y, p.w, p.h);
                    }
                });
            }

            // ---- Reconcile native state with the config -------------------
            commands::apply_config(&handle, state::get().config_snapshot());

            // ---- Debug/test simulation harness (ORBIT_SIMULATE=1) ---------
            mouse::start_simulation_listener();

            // ---- First-run window -----------------------------------------
            if !std::env::args().any(|a| a == "--hidden") {
                tray::show_settings(&handle);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing settings hides to tray; the app keeps running.
            if window.label() == "settings" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::reload_config,
            commands::set_launcher_enabled,
            commands::set_start_with_windows,
            commands::pick_executable,
            commands::pick_icon,
            commands::pick_directory,
            commands::get_app_icon,
            commands::check_paths,
            commands::list_available_apps,
            commands::launch_application,
            commands::quit_app
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { .. } | RunEvent::Exit = event {
                // Graceful shutdown: release the mouse hook before exit.
                mouse::uninstall();
                logger::info("Orbit exited cleanly");
            }
        });
}
