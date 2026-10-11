//! System tray: icon, context menu, and window toggling.

use tauri::menu::{CheckMenuItem, MenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::state;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("open-settings", "Open Settings")
        .check("toggle-enabled", "Launcher enabled")
        .separator()
        .text("reload", "Reload configuration")
        .text("about", "About")
        .separator()
        .text("quit", "Quit")
        .build()?;

    // Keep the check item handle so we can reflect state changes.
    let check_item: Option<CheckMenuItem<Wry>> = menu
        .get("toggle-enabled")
        .and_then(|i| i.as_check_menuitem().cloned());
    {
        let st = state::get();
        let mut slot = match st.tray_enabled_item.lock() {
            Ok(s) => s,
            Err(e) => e.into_inner(),
        };
        *slot = check_item;
    }

    let handle_menu = app.clone();
    let handle_icon = app.clone();

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("RRRex — radial launcher (hold the middle mouse button)");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder
        .on_menu_event(move |_tray, event| {
            let id: &str = event.id().as_ref();
            handle_menu_event(&handle_menu, id);
        })
        .on_tray_icon_event(move |_tray, event| {
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    toggle_settings(&handle_icon);
                }
            }
        })
        .build(app)?;

    crate::logger::info("tray icon created");
    Ok(())
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "open-settings" => show_settings(app),
        "toggle-enabled" => {
            let st = state::get();
            let mut cfg = st.config_snapshot();
            cfg.settings.enabled = !cfg.settings.enabled;
            let _ = crate::commands::apply_config(app, cfg);
        }
        "reload" => {
            let _ = crate::commands::reload_config_inner(app);
            crate::logger::info("configuration reloaded from tray");
        }
        "about" => {
            show_settings(app);
            let _ = app.emit_to(
                "settings",
                "app://navigate",
                serde_json::json!({ "tab": "about" }),
            );
        }
        "quit" => {
            crate::logger::info("quit from tray");
            crate::mouse::uninstall();
            app.exit(0);
        }
        _ => {}
    }
}

pub fn show_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn toggle_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

// Wry is the desktop runtime; used for the CheckMenuItem handle type.
use tauri::Wry;
