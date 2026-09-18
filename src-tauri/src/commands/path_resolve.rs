use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 相对路径基准计算结果（相对设置中的「路径根目录」）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelPathResult {
    pub rel_path: Option<String>,
    pub root_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResolveEffectivePathResult {
    Ok {
        path: String,
        used_relative: bool,
    },
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCheckItem {
    pub id: String,
    /// launcher | file | folder | url
    pub kind: String,
    pub path: String,
    pub rel_path: Option<String>,
    pub root_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCheckResult {
    pub id: String,
    /// ok | recoverable | missing | skipped
    pub status: String,
    pub resolved: Option<String>,
}

fn slashify(p: &str) -> String {
    p.replace('/', "\\")
}

/// 仅当 abs 落在某个 root 之下时，写出相对路径与所用根。
/// 多根同时匹配时取最长根；拒绝空相对段与 `..`。
pub fn compute_rel_path(abs: &str, roots: &[String]) -> RelPathResult {
    let empty = RelPathResult {
        rel_path: None,
        root_hint: None,
    };
    let abs_slash = slashify(abs);
    if abs_slash.trim().is_empty() {
        return empty;
    }
    // 网址等非本地路径不参与
    let lower = abs_slash.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("file://") {
        return empty;
    }

    let abs_key = abs_slash.to_lowercase();
    let mut best: Option<(usize, String, String)> = None;

    for root in roots {
        let root_slash = slashify(root);
        let root_core = root_slash.trim_end_matches('\\');
        if root_core.is_empty() {
            continue;
        }
        let root_key = root_core.to_lowercase();
        let prefix = format!("{root_key}\\");
        if abs_key.len() <= prefix.len() || !abs_key.starts_with(prefix.as_str()) {
            continue;
        }
        // normalize 只做大小写与分隔符替换，字符长度与原文一致，可按下标切片
        let rel = match abs_slash.get(root_core.len() + 1..) {
            Some(r) => r.to_string(),
            None => continue,
        };
        if rel.is_empty() || rel.split(['\\', '/']).any(|s| s == "..") {
            continue;
        }
        let score = root_key.len();
        let better = match &best {
            Some((s, _, _)) => score > *s,
            None => true,
        };
        if better {
            best = Some((score, rel, root_core.to_string()));
        }
    }

    match best {
        Some((_, rel, root)) => RelPathResult {
            rel_path: Some(rel),
            root_hint: Some(root),
        },
        None => empty,
    }
}

/// 绝对优先；失效则用 rootHint / 各根 + rel 回退。
pub fn resolve_effective_path(
    path: &str,
    rel_path: Option<&str>,
    root_hint: Option<&str>,
    roots: &[String],
) -> ResolveEffectivePathResult {
    let path_trim = path.trim();
    if !path_trim.is_empty() && Path::new(path_trim).exists() {
        return ResolveEffectivePathResult::Ok {
            path: path_trim.to_string(),
            used_relative: false,
        };
    }

    let Some(rel) = rel_path.map(str::trim).filter(|r| !r.is_empty()) else {
        return ResolveEffectivePathResult::Missing;
    };
    if rel.split(['\\', '/']).any(|s| s == "..") {
        return ResolveEffectivePathResult::Missing;
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(hint) = root_hint.map(str::trim).filter(|h| !h.is_empty()) {
        candidates.push(Path::new(hint).join(rel));
    }
    for root in roots {
        let root_trim = root.trim();
        if root_trim.is_empty() {
            continue;
        }
        let c = Path::new(root_trim).join(rel);
        if !candidates.contains(&c) {
            candidates.push(c);
        }
    }

    for c in candidates {
        if c.exists() {
            return ResolveEffectivePathResult::Ok {
                path: c.to_string_lossy().to_string(),
                used_relative: true,
            };
        }
    }
    ResolveEffectivePathResult::Missing
}

pub fn check_paths(items: &[PathCheckItem], roots: &[String]) -> Vec<PathCheckResult> {
    items
        .iter()
        .map(|item| {
            let kind = item.kind.to_lowercase();
            if kind == "url" {
                return PathCheckResult {
                    id: item.id.clone(),
                    status: "skipped".into(),
                    resolved: None,
                };
            }
            match resolve_effective_path(
                &item.path,
                item.rel_path.as_deref(),
                item.root_hint.as_deref(),
                roots,
            ) {
                ResolveEffectivePathResult::Ok {
                    path,
                    used_relative: false,
                } => PathCheckResult {
                    id: item.id.clone(),
                    status: "ok".into(),
                    resolved: Some(path),
                },
                ResolveEffectivePathResult::Ok {
                    path,
                    used_relative: true,
                } => PathCheckResult {
                    id: item.id.clone(),
                    status: "recoverable".into(),
                    resolved: Some(path),
                },
                ResolveEffectivePathResult::Missing => PathCheckResult {
                    id: item.id.clone(),
                    status: "missing".into(),
                    resolved: None,
                },
            }
        })
        .collect()
}

#[tauri::command]
pub fn compute_rel_path_cmd(path: String, roots: Vec<String>) -> RelPathResult {
    compute_rel_path(&path, &roots)
}

#[tauri::command]
pub fn resolve_effective_path_cmd(
    path: String,
    rel_path: Option<String>,
    root_hint: Option<String>,
    roots: Vec<String>,
) -> ResolveEffectivePathResult {
    resolve_effective_path(&path, rel_path.as_deref(), root_hint.as_deref(), &roots)
}

#[tauri::command]
pub fn check_paths_cmd(items: Vec<PathCheckItem>, roots: Vec<String>) -> Vec<PathCheckResult> {
    check_paths(&items, &roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn rel_path_under_longest_root() {
        let r = compute_rel_path(
            r"D:\Portable\Apps\Foo\Foo.exe",
            &roots(&[r"D:\Portable", r"D:\Portable\Apps"]),
        );
        assert_eq!(r.rel_path.as_deref(), Some(r"Foo\Foo.exe"));
        assert_eq!(r.root_hint.as_deref(), Some(r"D:\Portable\Apps"));
    }

    #[test]
    fn rel_path_outside_roots_is_null() {
        let r = compute_rel_path(r"C:\Windows\notepad.exe", &roots(&[r"D:\Portable"]));
        assert!(r.rel_path.is_none());
        assert!(r.root_hint.is_none());
    }

    #[test]
    fn rel_path_rejects_dotdot() {
        // 构造一个前缀匹配但相对段含 .. 的输入（手工字符串，不依赖真实 FS）
        let r = compute_rel_path(r"D:\Portable\..\Secret\a.exe", &roots(&[r"D:\Portable"]));
        assert!(r.rel_path.is_none());
    }

    #[test]
    fn rel_path_case_and_slash_insensitive() {
        let r = compute_rel_path(r"d:/portable/apps/foo.exe", &roots(&[r"D:\Portable\"]));
        assert_eq!(r.rel_path.as_deref(), Some(r"apps\foo.exe"));
        assert_eq!(r.root_hint.as_deref(), Some(r"D:\Portable"));
    }

    #[test]
    fn rel_path_skips_url() {
        let r = compute_rel_path("https://example.com", &roots(&[r"D:\Portable"]));
        assert!(r.rel_path.is_none());
    }

    #[test]
    fn resolve_prefers_existing_absolute() {
        let sys = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let notepad = Path::new(&sys).join("System32").join("notepad.exe");
        if !notepad.exists() {
            return;
        }
        let path = notepad.to_string_lossy().to_string();
        let r = resolve_effective_path(&path, Some(r"nope\nope.exe"), None, &roots(&[r"D:\Nope"]));
        match r {
            ResolveEffectivePathResult::Ok {
                used_relative: false,
                ..
            } => {}
            _ => panic!("expected absolute ok"),
        }
    }

    #[test]
    fn resolve_falls_back_to_relative() {
        let tmp = std::env::temp_dir().join("loft-path-resolve-test");
        let _ = std::fs::create_dir_all(&tmp);
        let file = tmp.join("sample.txt");
        std::fs::write(&file, b"x").unwrap();
        let root = tmp.to_string_lossy().to_string();
        let missing_abs = r"Z:\definitely\missing\sample.txt";
        let r = resolve_effective_path(
            missing_abs,
            Some("sample.txt"),
            Some(&root),
            &roots(&[root.as_str()]),
        );
        match r {
            ResolveEffectivePathResult::Ok {
                path,
                used_relative: true,
            } => {
                assert!(Path::new(&path).exists());
            }
            _ => panic!("expected relative fallback"),
        }
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn check_skips_url_and_flags_missing() {
        let items = vec![
            PathCheckItem {
                id: "u1".into(),
                kind: "url".into(),
                path: "https://example.com".into(),
                rel_path: None,
                root_hint: None,
            },
            PathCheckItem {
                id: "m1".into(),
                kind: "file".into(),
                path: r"Z:\no\such\file.exe".into(),
                rel_path: None,
                root_hint: None,
            },
        ];
        let out = check_paths(&items, &roots(&[]));
        assert_eq!(out[0].status, "skipped");
        assert_eq!(out[1].status, "missing");
    }
}
