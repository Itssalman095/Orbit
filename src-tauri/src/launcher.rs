//! Native application launching.
//!
//! Security model: the frontend never passes an executable path or shell
//! string — only an application id that is looked up in the validated,
//! persisted configuration. Launching uses Win32 process APIs directly
//! (CreateProcess for .exe, ShellExecuteEx only for .lnk shortcuts, which
//! CreateProcess cannot start). No `cmd.exe`, no string parsing.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::config::LauncherApp;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum LaunchError {
    #[serde(rename = "missingExecutable")]
    MissingExecutable,
    #[serde(rename = "permissionDenied")]
    PermissionDenied,
    #[serde(rename = "invalidWorkingDirectory")]
    InvalidWorkingDirectory,
    #[serde(rename = "spawnFailed")]
    SpawnFailed { message: String },
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::MissingExecutable => write!(f, "the executable no longer exists"),
            LaunchError::PermissionDenied => write!(f, "Windows denied access to the executable"),
            LaunchError::InvalidWorkingDirectory => write!(f, "the working directory is invalid"),
            LaunchError::SpawnFailed { message } => write!(f, "could not start process: {message}"),
        }
    }
}

/// Validates and launches a configured app. Never blocks for the child's
/// lifetime; spawn is fire-and-forget.
pub fn launch(app: &LauncherApp) -> Result<(), LaunchError> {
    let path = Path::new(&app.executable_path);
    if !path.is_file() {
        return Err(LaunchError::MissingExecutable);
    }

    if let Some(wd) = app.working_directory.as_deref() {
        if !Path::new(wd).is_dir() {
            return Err(LaunchError::InvalidWorkingDirectory);
        }
    }

    let args: Vec<String> = app.arguments.clone().unwrap_or_default();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    let result = if ext == "lnk" {
        shell_execute(path, &args, app.working_directory.as_deref())
    } else {
        create_process(path, &args, app.working_directory.as_deref())
    };

    crate::logger::info(&format!(
        "launch '{}' -> {}",
        app.name,
        match &result {
            Ok(()) => "ok".into(),
            Err(e) => format!("error: {e}"),
        }
    ));
    result
}

fn create_process(path: &Path, args: &[String], wd: Option<&str>) -> Result<(), LaunchError> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut cmd = std::process::Command::new(path);
    cmd.args(args);
    if let Some(wd) = wd {
        cmd.current_dir(wd);
    }
    #[cfg(windows)]
    {
        // Don't flash a console window for console-subsystem executables.
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd.spawn().map(|_| ()).map_err(map_io_error)
}

/// ShellExecuteExW is required for .lnk shortcuts (CreateProcess cannot run
/// them). Parameters are passed as a structured argv, never through a shell.
fn shell_execute(path: &Path, args: &[String], wd: Option<&str>) -> Result<(), LaunchError> {
    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SHELLEXECUTEINFOW};
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let quoted_args: Vec<String> = args
            .iter()
            .map(|a| {
                if a.contains(' ') || a.contains('"') {
                    format!("\"{}\"", a.replace('"', ""))
                } else {
                    a.clone()
                }
            })
            .collect();

        unsafe {
            let file = HSTRING::from(path.as_os_str().to_str().unwrap_or_default());
            let params = if quoted_args.is_empty() {
                None
            } else {
                Some(HSTRING::from(quoted_args.join(" ")))
            };
            let dir = wd
                .map(|d| HSTRING::from(d))
                .unwrap_or_else(|| HSTRING::from(""));

            let mut sei = SHELLEXECUTEINFOW {
                cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                fMask: SEE_MASK_FLAG_NO_UI,
                nShow: SW_SHOWNORMAL.0 as i32,
                lpFile: windows::core::PCWSTR(file.as_ptr()),
                lpParameters: params
                    .as_ref()
                    .map(|p| windows::core::PCWSTR(p.as_ptr()))
                    .unwrap_or(windows::core::PCWSTR::null()),
                lpDirectory: if dir.is_empty() {
                    windows::core::PCWSTR::null()
                } else {
                    windows::core::PCWSTR(dir.as_ptr())
                },
                ..Default::default()
            };

            ShellExecuteExW(&mut sei).map_err(|e| LaunchError::SpawnFailed {
                message: e.to_string(),
            })
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (path, args, wd);
        Err(LaunchError::SpawnFailed {
            message: "not supported on this platform".into(),
        })
    }
}

fn map_io_error(e: std::io::Error) -> LaunchError {
    match e.kind() {
        std::io::ErrorKind::NotFound => LaunchError::MissingExecutable,
        std::io::ErrorKind::PermissionDenied => LaunchError::PermissionDenied,
        _ => LaunchError::SpawnFailed {
            message: e.to_string(),
        },
    }
}

/// Launch + report failures to the settings window as a toast event.
pub fn launch_and_report(app: &AppHandle, cfg: &LauncherApp) {
    if let Err(e) = launch(cfg) {
        crate::logger::error(&format!("launch failed for '{}': {e}", cfg.name));
        let payload = serde_json::json!({
            "appId": cfg.id,
            "appName": cfg.name,
            "message": e.to_string(),
        });
        let _ = app.emit_to("settings", "app://launch-error", payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(exe: &str, args: Option<Vec<String>>, wd: Option<&str>) -> LauncherApp {
        LauncherApp {
            id: "t".into(),
            name: "T".into(),
            executable_path: exe.into(),
            icon_path: None,
            arguments: args,
            working_directory: wd.map(str::to_string),
            enabled: true,
        }
    }

    #[test]
    fn missing_executable_is_rejected() {
        let a = app("C:\\definitely\\does\\not\\exist-12345\\x.exe", None, None);
        match launch(&a) {
            Err(LaunchError::MissingExecutable) => {}
            other => panic!("expected MissingExecutable, got {other:?}"),
        }
    }

    #[test]
    fn invalid_working_directory_is_rejected() {
        let exe = std::env::current_exe().unwrap();
        let a = app(
            exe.to_str().unwrap(),
            None,
            Some("C:\\definitely\\no\\dir-98765"),
        );
        match launch(&a) {
            Err(LaunchError::InvalidWorkingDirectory) => {}
            other => panic!("expected InvalidWorkingDirectory, got {other:?}"),
        }
    }

    #[test]
    fn launch_requires_a_file() {
        let a = app("C:\\Windows", None, None); // a directory, not a file
        assert!(matches!(launch(&a), Err(LaunchError::MissingExecutable)));
    }
}
