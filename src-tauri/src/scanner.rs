//! Installed-app discovery.
//!
//! Scans the Windows Start Menu and Desktop shortcut folders (`.lnk` files)
//! so the settings UI can offer a searchable "your installed apps" catalog
//! instead of forcing users to hunt for executable paths. `.lnk` files are
//! first-class launch targets in Orbit (resolved via ShellExecuteEx and the
//! shell icon factory), so a Start Menu shortcut is all we need.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableApp {
    /// Unique id (the shortcut path itself).
    pub id: String,
    pub name: String,
    pub path: String,
}

/// Shortcut names that are clearly not applications.
const SKIP_NAME_TOKENS: &[&str] = &["uninstall", "uninst ", "un install", "remove ", "repair"];

const MAX_DEPTH: u32 = 5;

/// Scans the standard shortcut locations (user + common Start Menu, user +
/// public Desktop) and returns a deduplicated, alphabetized catalog.
pub fn scan_installed_apps() -> Vec<AvailableApp> {
    let mut roots = Vec::new();

    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(
            PathBuf::from(&appdata)
                .join(r"Microsoft\Windows\Start Menu")
                .join("Programs"),
        );
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(
            PathBuf::from(&program_data)
                .join(r"Microsoft\Windows\Start Menu")
                .join("Programs"),
        );
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        roots.push(PathBuf::from(&profile).join("Desktop"));
    }
    if let Some(public) = std::env::var_os("PUBLIC") {
        roots.push(PathBuf::from(&public).join("Desktop"));
    }

    scan_roots(&roots)
}

/// Pure core (unit tested): collects shortcuts from the given roots.
pub fn scan_roots(roots: &[PathBuf]) -> Vec<AvailableApp> {
    let mut out: Vec<AvailableApp> = Vec::new();
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut seen_paths: HashSet<String> = HashSet::new();

    for root in roots {
        collect_dir(root, 0, &mut out, &mut seen_names, &mut seen_paths);
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn collect_dir(
    dir: &Path,
    depth: u32,
    out: &mut Vec<AvailableApp>,
    seen_names: &mut HashSet<String>,
    seen_paths: &mut HashSet<String>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if file_name.starts_with('.') || file_name.starts_with('~') {
            continue;
        }
        let path = entry.path();
        let lower = file_name.to_lowercase();

        if path.is_dir() {
            // Skip maintenance folders that live inside Start Menu trees.
            if lower == "startup" || lower.contains("uninstall") || lower.contains("maintenance") {
                continue;
            }
            collect_dir(&path, depth + 1, out, seen_names, seen_paths);
        } else if lower.ends_with(".lnk") {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let name = clean_name(stem);
            if name.is_empty() || is_maintenance_link(&name) {
                continue;
            }
            let path_str = path.to_string_lossy().into_owned();
            if !seen_paths.insert(path_str.to_lowercase()) {
                continue;
            }
            if !seen_names.insert(name.to_lowercase()) {
                continue; // same app from another Start Menu location
            }
            out.push(AvailableApp {
                id: path_str.clone(),
                name,
                path: path_str,
            });
        }
    }
}

/// Cleans a shortcut file stem into a display name.
pub fn clean_name(stem: &str) -> String {
    // Trim the "(x86)"/"- shortcut" style noise Windows sometimes leaves.
    let trimmed = stem.trim();
    let trimmed = trimmed
        .trim_end_matches(" - shortcut")
        .trim_end_matches(" - Shortcut");
    trimmed.trim().to_string()
}

/// True for names that clearly point to uninstallers/repairs.
pub fn is_maintenance_link(name: &str) -> bool {
    let lower = name.to_lowercase();
    SKIP_NAME_TOKENS.iter().any(|t| lower.contains(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_tree(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orbit-scan-{}-{}",
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(dir: &Path, rel: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "").unwrap();
    }

    #[test]
    fn collects_shortcuts_recursively_and_sorted() {
        let root = temp_tree("sort");
        touch(&root, "Programs/Chrome.lnk");
        touch(&root, "Programs/Accessories/Notepad.lnk");
        touch(&root, "Programs/AAA App.lnk");

        let apps = scan_roots(&[root.join("Programs")]);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["AAA App", "Chrome", "Notepad"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn skips_uninstallers_startup_and_non_shortcuts() {
        let root = temp_tree("skip");
        touch(&root, "Programs/Good App.lnk");
        touch(&root, "Programs/Uninstall Foo.lnk");
        touch(&root, "Programs/Repair Bar.lnk");
        touch(&root, "Programs/Startup/AutoStart.lnk");
        touch(&root, "Programs/readme.txt");

        let apps = scan_roots(&[root.join("Programs")]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Good App");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dedupes_same_name_across_roots() {
        let a = temp_tree("dup-a");
        let b = temp_tree("dup-b");
        touch(&a, "App.lnk");
        touch(&b, "App.lnk");
        touch(&b, "Other.lnk");

        let apps = scan_roots(&[a.clone(), b.clone()]);
        let names: Vec<&str> = apps.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, vec!["App", "Other"]);
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }

    #[test]
    fn ignores_hidden_and_temp_files() {
        let root = temp_tree("hidden");
        touch(&root, "Programs/.hidden.lnk");
        touch(&root, "Programs/~tmp.lnk");
        touch(&root, "Programs/Visible.lnk");

        let apps = scan_roots(&[root.join("Programs")]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Visible");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cleans_names() {
        assert_eq!(clean_name("Google Chrome"), "Google Chrome");
        assert_eq!(clean_name("Thing - Shortcut"), "Thing");
        assert_eq!(clean_name(" Thing - shortcut "), "Thing");
    }

    #[test]
    fn empty_roots_give_empty_catalog() {
        assert!(scan_roots(&[]).is_empty());
        let missing = PathBuf::from(r"C:\definitely\does\not\exist-orbit");
        assert!(scan_roots(&[missing]).is_empty());
    }
}
