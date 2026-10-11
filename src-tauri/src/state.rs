//! Global shared state, initialized once during app setup.
//!
//! A process-wide singleton is used (the single-instance plugin guarantees
//! one process) so the mouse hook thread and Tauri commands can reach the
//! same state without lifetime plumbing through raw Win32 callbacks.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use tauri::menu::CheckMenuItem;
use tauri::Wry;

use crate::config::Config;
use crate::menu::MenuRuntime;

pub struct Paths {
    pub config_dir: PathBuf,
    pub icon_cache_dir: PathBuf,
}

pub struct SharedState {
    pub config: RwLock<Config>,
    /// Menu runtime (phase, geometry snapshot, selection). Owned by the hook
    /// thread while a menu interaction is in flight; other threads only
    /// read/peek (e.g. the rendered-ack listener consumes `pending_show`).
    pub menu: Mutex<MenuRuntime>,
    /// source path -> PNG data URL (memory layer of the icon cache).
    pub icons: Mutex<HashMap<String, String>>,
    pub overlay_ready: AtomicBool,
    pub tray_enabled_item: Mutex<Option<CheckMenuItem<Wry>>>,
    pub paths: Paths,
}

impl SharedState {
    /// Clones the current config. Poisoned locks are recovered.
    pub fn config_snapshot(&self) -> Config {
        match self.config.read() {
            Ok(c) => c.clone(),
            Err(e) => e.into_inner().clone(),
        }
    }

    pub fn set_config(&self, cfg: Config) {
        match self.config.write() {
            Ok(mut c) => *c = cfg,
            Err(e) => *e.into_inner() = cfg,
        }
    }

    pub fn icon_for(&self, source: &str) -> Option<String> {
        match self.icons.lock() {
            Ok(m) => m.get(source).cloned(),
            Err(e) => e.into_inner().get(source).cloned(),
        }
    }

    pub fn insert_icon(&self, source: String, data_url: String) {
        match self.icons.lock() {
            Ok(mut m) => {
                m.insert(source, data_url);
            }
            Err(e) => {
                e.into_inner().insert(source, data_url);
            }
        }
    }
}

static SHARED: OnceLock<Arc<SharedState>> = OnceLock::new();

pub fn init(state: Arc<SharedState>) {
    if SHARED.set(state).is_err() {
        crate::logger::warn("shared state initialized twice");
    }
}

/// Panics if called before setup finished — after that it is always set.
pub fn get() -> Arc<SharedState> {
    SHARED
        .get()
        .expect("SharedState must be initialized during setup")
        .clone()
}
