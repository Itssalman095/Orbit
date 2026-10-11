//! Configuration model, validation and JSON persistence.
//!
//! The JSON file lives in the OS app-data directory. Loaded data is never
//! trusted: `sanitize_config` repairs missing fields, wrong types and
//! out-of-range values, mirroring `src/lib/configValidation.ts`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TriggerButton {
    #[default]
    Middle,
    X1,
    X2,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherApp {
    pub id: String,
    pub name: String,
    pub executable_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for LauncherApp {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            executable_path: String::new(),
            icon_path: None,
            arguments: None,
            working_directory: None,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub trigger: TriggerButton,
    #[serde(default = "default_radius")]
    pub radius: f64,
    #[serde(default = "default_icon_size")]
    pub icon_size: f64,
    #[serde(default = "default_dead_zone")]
    pub dead_zone_radius: f64,
    #[serde(default = "default_start_angle")]
    pub start_angle_deg: f64,
    #[serde(default = "default_anim")]
    pub animation_speed: f64,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_opacity")]
    pub menu_opacity: f64,
    #[serde(default = "default_true")]
    pub selection_glow: bool,
    #[serde(default = "default_hysteresis")]
    pub hysteresis_deg: f64,
    #[serde(default = "default_true")]
    pub launch_on_release: bool,
    #[serde(default)]
    pub sticky_when_no_selection: bool,
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default = "default_true")]
    pub start_minimized: bool,
}

fn default_radius() -> f64 {
    140.0
}
fn default_icon_size() -> f64 {
    56.0
}
fn default_dead_zone() -> f64 {
    30.0
}
fn default_start_angle() -> f64 {
    -90.0
}
fn default_anim() -> f64 {
    120.0
}
fn default_opacity() -> f64 {
    0.9
}
fn default_hysteresis() -> f64 {
    2.5
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger: TriggerButton::Middle,
            radius: default_radius(),
            icon_size: default_icon_size(),
            dead_zone_radius: default_dead_zone(),
            start_angle_deg: default_start_angle(),
            animation_speed: default_anim(),
            theme: Theme::Dark,
            menu_opacity: default_opacity(),
            selection_glow: true,
            hysteresis_deg: default_hysteresis(),
            launch_on_release: true,
            sticky_when_no_selection: false,
            start_with_windows: false,
            start_minimized: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub applications: Vec<LauncherApp>,
}

fn default_version() -> u32 {
    CONFIG_VERSION
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            settings: Settings::default(),
            applications: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Sanitization
// ---------------------------------------------------------------------------

fn clamp_f(v: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        fallback
    }
}

fn sanitize_f(value: &serde_json::Value, min: f64, max: f64, fallback: f64) -> f64 {
    clamp_f(value.as_f64().unwrap_or(fallback), min, max, fallback)
}

fn sanitize_bool(value: &serde_json::Value, fallback: bool) -> bool {
    value.as_bool().unwrap_or(fallback)
}

fn non_empty(s: String) -> Option<String> {
    let t = s.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

pub fn name_from_path(path: &str) -> String {
    let clean = path.trim_end_matches(['\\', '/']);
    let base = clean.rsplit(['\\', '/']).next().unwrap_or(clean);
    let stripped = base
        .strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".EXE"))
        .or_else(|| base.strip_suffix(".lnk"))
        .or_else(|| base.strip_suffix(".LNK"))
        .unwrap_or(base);
    if stripped.is_empty() {
        base.to_string()
    } else {
        stripped.to_string()
    }
}

/// Repairs a raw JSON value into a valid Config. Never fails.
pub fn sanitize_config(raw: &serde_json::Value) -> Config {
    let root = match raw {
        serde_json::Value::Object(m) => m,
        _ => return Config::default(),
    };

    let d = &serde_json::Value::Null;
    let empty_map = serde_json::Map::new();
    let s = root
        .get("settings")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty_map);

    let trigger = s
        .get("trigger")
        .and_then(|v| serde_json::from_value::<TriggerButton>(v.clone()).ok())
        .unwrap_or(TriggerButton::Middle);
    let theme = s
        .get("theme")
        .and_then(|v| serde_json::from_value::<Theme>(v.clone()).ok())
        .unwrap_or(Theme::Dark);

    let settings = Settings {
        enabled: sanitize_bool(s.get("enabled").unwrap_or(d), true),
        trigger,
        radius: sanitize_f(s.get("radius").unwrap_or(d), 60.0, 400.0, default_radius()),
        icon_size: sanitize_f(
            s.get("iconSize").unwrap_or(d),
            24.0,
            128.0,
            default_icon_size(),
        ),
        dead_zone_radius: sanitize_f(
            s.get("deadZoneRadius").unwrap_or(d),
            0.0,
            200.0,
            default_dead_zone(),
        ),
        start_angle_deg: sanitize_f(
            s.get("startAngleDeg").unwrap_or(d),
            -360.0,
            360.0,
            default_start_angle(),
        ),
        animation_speed: sanitize_f(
            s.get("animationSpeed").unwrap_or(d),
            0.0,
            600.0,
            default_anim(),
        ),
        theme,
        menu_opacity: sanitize_f(
            s.get("menuOpacity").unwrap_or(d),
            0.2,
            1.0,
            default_opacity(),
        ),
        selection_glow: sanitize_bool(s.get("selectionGlow").unwrap_or(d), true),
        hysteresis_deg: sanitize_f(
            s.get("hysteresisDeg").unwrap_or(d),
            0.0,
            15.0,
            default_hysteresis(),
        ),
        launch_on_release: sanitize_bool(s.get("launchOnRelease").unwrap_or(d), true),
        sticky_when_no_selection: sanitize_bool(s.get("stickyWhenNoSelection").unwrap_or(d), false),
        start_with_windows: sanitize_bool(s.get("startWithWindows").unwrap_or(d), false),
        start_minimized: sanitize_bool(s.get("startMinimized").unwrap_or(d), true),
    };

    let mut applications = Vec::new();
    let mut seen = std::collections::HashSet::new();
    if let Some(items) = root.get("applications").and_then(|v| v.as_array()) {
        for item in items {
            let obj = match item.as_object() {
                Some(o) => o,
                None => continue,
            };
            let id = obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());
            let exe = obj
                .get("executablePath")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());
            let (id, exe) = match (id, exe) {
                (Some(i), Some(e)) => (i.to_string(), e.to_string()),
                _ => continue,
            };
            if !seen.insert(id.clone()) {
                continue; // duplicate id
            }
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .and_then(|v| non_empty(v.to_string()))
                .unwrap_or_else(|| name_from_path(&exe));
            let arguments = obj.get("arguments").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| non_empty(s.to_string()))
                    .collect::<Vec<_>>()
            });
            applications.push(LauncherApp {
                id,
                name,
                executable_path: exe,
                icon_path: obj
                    .get("iconPath")
                    .and_then(|v| v.as_str())
                    .and_then(|v| non_empty(v.to_string())),
                arguments,
                working_directory: obj
                    .get("workingDirectory")
                    .and_then(|v| v.as_str())
                    .and_then(|v| non_empty(v.to_string())),
                enabled: obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            });
        }
    }

    Config {
        version: CONFIG_VERSION,
        settings,
        applications,
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

pub const CONFIG_FILE: &str = "config.json";

/// Loads the config from `dir`, repairing corruption:
/// a malformed file is renamed to `config.corrupt.json` and defaults are used.
pub fn load(dir: &Path) -> Config {
    let path = dir.join(CONFIG_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) => sanitize_config(&value),
            Err(e) => {
                crate::logger::warn(&format!("config file is corrupted: {e}; backing up"));
                let backup = dir.join("config.corrupt.json");
                let _ = fs::copy(&path, &backup);
                Config::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
        Err(e) => {
            crate::logger::warn(&format!("could not read config file ({e}); using defaults"));
            Config::default()
        }
    }
}

/// Atomically saves the config (write temp file, then rename over the target).
pub fn save(dir: &Path, config: &Config) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let path = dir.join(CONFIG_FILE);
    let tmp = dir.join(format!("{}.tmp", CONFIG_FILE));
    fs::write(&tmp, json + "\n")?;
    // On Windows, rename fails if the target exists; remove then rename.
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(&tmp, &path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("orbit-test-{}", std::process::id()));
        let sub = dir.join(format!(
            "cfg-{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&sub).unwrap();
        sub
    }

    #[test]
    fn defaults_for_garbage() {
        let cfg = sanitize_config(&json!(null));
        assert_eq!(cfg.version, CONFIG_VERSION);
        assert_eq!(cfg.settings.radius, 140.0);
        assert_eq!(cfg.settings.trigger, TriggerButton::Middle);
        assert!(cfg.applications.is_empty());
    }

    #[test]
    fn clamps_out_of_range() {
        let cfg = sanitize_config(&json!({
            "settings": { "radius": 99999.0, "iconSize": 1.0, "deadZoneRadius": -5.0, "menuOpacity": 3.0 }
        }));
        assert_eq!(cfg.settings.radius, 400.0);
        assert_eq!(cfg.settings.icon_size, 24.0);
        assert_eq!(cfg.settings.dead_zone_radius, 0.0);
        assert_eq!(cfg.settings.menu_opacity, 1.0);
    }

    #[test]
    fn wrong_types_fall_back() {
        let cfg = sanitize_config(&json!({
            "settings": { "enabled": "yes", "trigger": "banana", "radius": "140", "theme": 7 }
        }));
        assert!(cfg.settings.enabled);
        assert_eq!(cfg.settings.trigger, TriggerButton::Middle);
        assert_eq!(cfg.settings.radius, 140.0);
        assert_eq!(cfg.settings.theme, Theme::Dark);
    }

    #[test]
    fn drops_invalid_apps_and_dedupes() {
        let cfg = sanitize_config(&json!({
            "applications": [
                { "id": "ok", "name": "Ok", "executablePath": "C:\\ok.exe", "enabled": true },
                { "name": "no id", "executablePath": "C:\\x.exe", "enabled": true },
                { "id": "dup", "name": "First", "executablePath": "C:\\a.exe", "enabled": true },
                { "id": "dup", "name": "Second", "executablePath": "C:\\b.exe", "enabled": true },
                "junk"
            ]
        }));
        assert_eq!(cfg.applications.len(), 2);
        assert_eq!(cfg.applications[0].id, "ok");
        assert_eq!(cfg.applications[1].name, "First");
    }

    #[test]
    fn name_falls_back_to_path() {
        let cfg = sanitize_config(&json!({
            "applications": [{ "id": "a", "name": "", "executablePath": "C:\\Tools\\MyTool.exe", "enabled": true }]
        }));
        assert_eq!(cfg.applications[0].name, "MyTool");
    }

    #[test]
    fn roundtrip_save_load() {
        let dir = temp_dir();
        let mut cfg = Config::default();
        cfg.settings.radius = 220.5;
        cfg.applications.push(LauncherApp {
            id: "t1".into(),
            name: "Test".into(),
            executable_path: "C:\\test\\app.exe".into(),
            icon_path: None,
            arguments: Some(vec!["--a".into(), "b c".into()]),
            working_directory: Some("C:\\test".into()),
            enabled: false,
        });
        save(&dir, &cfg).unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.settings.radius, 220.5);
        assert_eq!(loaded.applications.len(), 1);
        assert_eq!(
            loaded.applications[0].arguments.as_deref().unwrap(),
            ["--a", "b c"]
        );
        assert!(!loaded.applications[0].enabled);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupted_file_backs_up_and_defaults() {
        let dir = temp_dir();
        fs::write(dir.join(CONFIG_FILE), "{ not valid json !!!").unwrap();
        let cfg = load(&dir);
        assert_eq!(cfg.settings.radius, 140.0);
        assert!(dir.join("config.corrupt.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_uses_defaults() {
        let dir = temp_dir();
        let cfg = load(&dir);
        assert!(cfg.applications.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
