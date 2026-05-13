use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShortcutEntry {
    pub name: String,
    pub path: String,
    pub target: Option<String>,
    pub icon_path: Option<String>,
    pub working_dir: Option<String>,
}

fn start_menu_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(all) = std::env::var("ProgramData") {
        out.push(PathBuf::from(all).join("Microsoft/Windows/Start Menu/Programs"));
    } else {
        out.push(PathBuf::from("C:/ProgramData/Microsoft/Windows/Start Menu/Programs"));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        out.push(PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs"));
    }
    out
}

#[tauri::command]
pub fn scan_start_menu() -> Result<Vec<ShortcutEntry>, String> {
    let mut items: Vec<ShortcutEntry> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for root in start_menu_roots() {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .max_depth(6)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !ext.eq_ignore_ascii_case("lnk") && !ext.eq_ignore_ascii_case("url") {
                continue;
            }
            let name = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            // de-dupe by lowercased name (Start Menu often has same name twice for user/all-users)
            let dedup_key = name.to_lowercase();
            if seen.contains(&dedup_key) {
                continue;
            }
            seen.insert(dedup_key);

            let resolved = resolve(p);
            items.push(ShortcutEntry {
                name,
                path: p.to_string_lossy().to_string(),
                target: resolved.0,
                icon_path: resolved.1,
                working_dir: resolved.2,
            });
        }
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(items)
}

#[tauri::command]
pub fn resolve_shortcut(path: String) -> Result<ShortcutEntry, String> {
    let p = Path::new(&path);
    let name = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("(未知)")
        .to_string();
    let (target, icon_path, working_dir) = resolve(p);
    Ok(ShortcutEntry {
        name,
        path: path.clone(),
        target,
        icon_path,
        working_dir,
    })
}

#[cfg(windows)]
fn resolve(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    use lnk::ShellLink;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if !ext.eq_ignore_ascii_case("lnk") {
        return (None, None, None);
    }
    // lnk 0.5.1 panics on some valid shortcuts (e.g. unwraps a missing
    // ShowCommand). Catch the panic so a single bad file doesn't kill scanning.
    let parsed = catch_unwind(AssertUnwindSafe(|| ShellLink::open(path)));
    match parsed {
        Ok(Ok(link)) => {
            let target = link
                .link_info()
                .as_ref()
                .and_then(|i| i.local_base_path().clone());
            let icon = link.icon_location().clone();
            let working = link.working_dir().clone();
            (target, icon, working)
        }
        _ => (None, None, None),
    }
}

#[cfg(not(windows))]
fn resolve(_path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    (None, None, None)
}

#[tauri::command]
pub fn launch_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| format!("启动失败: {e}"))
}
