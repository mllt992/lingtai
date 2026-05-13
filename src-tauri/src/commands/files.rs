use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn open_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| format!("打开失败: {e}"))
}

#[tauri::command]
pub fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("打开网址失败: {e}"))
}

#[tauri::command]
pub fn reveal_in_explorer(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let target = if p.is_file() {
        p.parent()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or(path.clone())
    } else {
        path.clone()
    };
    app.opener()
        .open_path(&target, None::<&str>)
        .map_err(|e| format!("打开资源管理器失败: {e}"))
}
