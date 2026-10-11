//! RRRex — a radial application launcher for Windows.
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
const APP_IDENTIFIER: &str = "com.rrrex.launcher";

/// Builds the shared state before any webview can issue commands.
///
/// Tauri creates config windows (and starts loading their webviews) before
/// the setup hook runs, so IPC commands can arrive before setup() — the
/// state must therefore exist before the builder is even constructed.
/// The path mirrors tauri's `app_config_dir()` on Windows
/// (%APPDATA%\<identifier>); setup verifies they agree.
fn init_shared_state() -> std::sync::Arc<state::SharedState> {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| {
        // Extremely defensive: relative dir if the env var is missing.
        ".".to_string()
    });
    let config_dir = PathBuf::from(base).join(APP_IDENTIFIER);
    let icon_cache_dir = config_dir.join("icons");
    let _ = std::fs::create_dir_all(&config_dir);
    let _ = std::fs::create_dir_all(&icon_cache_dir);

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

            // ---- Paths & logging ------------------------------------------
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
            logger::init(st.paths.config_dir.join("rrrex.log"));
            logger::info("RRRex starting");

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

            // ---- Debug/test simulation harness (RRREX_SIMULATE=1) ---------
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
                logger::info("RRRex exited cleanly");
            }
        });
}
