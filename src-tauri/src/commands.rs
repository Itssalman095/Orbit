//! Tauri IPC commands — the only native surface exposed to the webview.
//! Each command is narrowly scoped and validates its inputs.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::config::{self, Config};
use crate::state;

/// Structured error surfaced to the frontend over IPC.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl IpcError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub config: Config,
    pub missing: HashMap<String, bool>,
}

pub fn compute_missing(cfg: &Config) -> HashMap<String, bool> {
    cfg.applications
        .iter()
        .map(|a| (a.id.clone(), !Path::new(&a.executable_path).is_file()))
        .collect()
}

/// Applies a sanitized config everywhere:
/// persist to disk, update shared state, reconcile the global hook,
/// reconcile Windows autostart, refresh the tray toggle, warm the icon cache.
pub fn apply_config(app: &AppHandle, cfg: Config) -> ConfigResponse {
    let st = state::get();

    if let Err(e) = config::save(&st.paths.config_dir, &cfg) {
        crate::logger::error(&format!("failed to persist config: {e}"));
    }
    st.set_config(cfg.clone());

    // Reconcile the global mouse hook with the enabled flag.
    if cfg.settings.enabled && !crate::mouse::is_installed() {
        crate::mouse::install(app.clone());
    } else if !cfg.settings.enabled && crate::mouse::is_installed() {
        crate::mouse::uninstall();
    }

    // Reconcile "start with Windows" (HKCU Run key, no admin rights).
    reconcile_autostart(app, cfg.settings.start_with_windows);

    // Reflect enabled state in the tray check item.
    {
        let slot = match st.tray_enabled_item.lock() {
            Ok(s) => s,
            Err(e) => e.into_inner(),
        };
        if let Some(item) = slot.as_ref() {
            let _ = item.set_checked(cfg.settings.enabled);
        }
    }

    // Refresh extracted icons in the background.
    crate::icons::warm_cache(st);

    let missing = compute_missing(&cfg);
    ConfigResponse {
        config: cfg,
        missing,
    }
}

fn reconcile_autostart(app: &AppHandle, want: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    match autolaunch.is_enabled() {
        Ok(current) => {
            if current != want {
                let result = if want {
                    autolaunch.enable()
                } else {
                    autolaunch.disable()
                };
                if let Err(e) = result {
                    crate::logger::error(&format!("autostart toggle failed: {e}"));
                } else {
                    crate::logger::info(&format!(
                        "start with windows {}",
                        if want { "enabled" } else { "disabled" }
                    ));
                }
            }
        }
        Err(e) => {
            crate::logger::warn(&format!("autostart state unavailable: {e}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn load_config() -> Result<ConfigResponse, IpcError> {
    let st = state::get();
    let cfg = st.config_snapshot();
    let missing = compute_missing(&cfg);
    Ok(ConfigResponse {
        config: cfg,
        missing,
    })
}

/// Accepts raw JSON: the backend sanitizes it so partially-invalid payloads
/// still produce a valid, persisted configuration.
#[tauri::command]
pub fn save_config(app: AppHandle, config: serde_json::Value) -> Result<ConfigResponse, IpcError> {
    let cfg = config::sanitize_config(&config);
    Ok(apply_config(&app, cfg))
}

pub fn reload_config_inner(app: &AppHandle) -> ConfigResponse {
    let st = state::get();
    let cfg = config::load(&st.paths.config_dir);
    let resp = apply_config(app, cfg);
    let _ = app.emit_to("settings", "app://config-reloaded", ());
    resp
}

#[tauri::command]
pub fn reload_config(app: AppHandle) -> Result<ConfigResponse, IpcError> {
    Ok(reload_config_inner(&app))
}

#[tauri::command]
pub fn set_launcher_enabled(app: AppHandle, enabled: bool) -> Result<ConfigResponse, IpcError> {
    let st = state::get();
    let mut cfg = st.config_snapshot();
    cfg.settings.enabled = enabled;
    Ok(apply_config(&app, cfg))
}

#[tauri::command]
pub fn set_start_with_windows(app: AppHandle, enabled: bool) -> Result<(), IpcError> {
    let st = state::get();
    let mut cfg = st.config_snapshot();
    cfg.settings.start_with_windows = enabled;
    let _ = apply_config(&app, cfg);
    Ok(())
}

/// Native Windows file picker for executables. Runs on the async runtime
/// (blocking dialogs must not touch the main thread).
#[tauri::command]
pub async fn pick_executable(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Applications", &["exe", "lnk"])
        .blocking_pick_file();
    picked
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn pick_icon(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app
        .dialog()
        .file()
        .add_filter("Icon images", &["png", "jpg", "jpeg", "ico", "bmp"])
        .blocking_pick_file();
    picked
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_folder();
    picked
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Extracts a Windows icon as a PNG data URL (cached).
#[tauri::command]
pub async fn get_app_icon(path: String) -> Option<String> {
    if path.len() > 4096 || !Path::new(&path).is_absolute() {
        return None; // basic input validation
    }
    let st = state::get();
    if let Some(cached) = st.icon_for(&path) {
        return Some(cached);
    }
    let url = crate::icons::get_icon_data_url(&path, &st.paths.icon_cache_dir);
    if let Some(u) = &url {
        st.insert_icon(path, u.clone());
    }
    url
}

#[tauri::command]
pub fn check_paths(paths: Vec<String>) -> Vec<bool> {
    paths.iter().map(|p| Path::new(p).is_file()).collect()
}

/// Installed-app catalog for the "add application" picker
/// (Start Menu + Desktop shortcuts, deduplicated).
#[tauri::command]
pub fn list_available_apps() -> Result<Vec<crate::scanner::AvailableApp>, IpcError> {
    Ok(crate::scanner::scan_installed_apps())
}

/// Launches a configured application by id. The frontend can never pass an
/// arbitrary executable: only stored, validated entries can be launched.
#[tauri::command]
pub fn launch_application(app: AppHandle, app_id: String) -> Result<(), IpcError> {
    if app_id.len() > 256 {
        return Err(IpcError::new("invalidId", "Invalid application id"));
    }
    let st = state::get();
    let cfg = st.config_snapshot();
    match cfg.applications.iter().find(|a| a.id == app_id) {
        None => Err(IpcError::new("notFound", "Unknown application id")),
        Some(target) => {
            let target = target.clone();
            std::thread::spawn(move || {
                crate::launcher::launch_and_report(&app, &target);
            });
            Ok(())
        }
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::logger::info("quit requested from settings");
    crate::mouse::uninstall();
    app.exit(0);
}

// Manager trait used for webview window lookups in future commands.
#[allow(unused_imports)]
use tauri::Manager as _Manager;
