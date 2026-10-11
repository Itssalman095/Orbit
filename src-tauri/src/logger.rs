//! Minimal file logger. Writes timestamped lines to the app-data log file
//! and echoes to stderr in debug builds. Deliberately dependency-free.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub fn init(path: PathBuf) {
    let _ = LOG_PATH.set(path);
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn log(level: &str, msg: &str) {
    let line = format!("[{} {:5}] {}\n", now_millis(), level, msg);
    if let Some(path) = LOG_PATH.get() {
        let _guard = WRITE_LOCK.lock();
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(line.as_bytes());
        }
    }
    if cfg!(debug_assertions) {
        eprint!("{line}");
    }
}

pub fn info(msg: &str) {
    log("INFO", msg);
}

pub fn warn(msg: &str) {
    log("WARN", msg);
}

pub fn error(msg: &str) {
    log("ERROR", msg);
}
