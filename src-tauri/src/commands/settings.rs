use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("获取配置目录失败: {e}"))?;
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    Ok(dir)
}

fn read_json(app: &AppHandle, file: &str, default: Value) -> Result<Value, String> {
    let path = config_dir(app)?.join(file);
    if !path.exists() {
        return Ok(default);
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("读取 {file} 失败: {e}"))?;
    if text.trim().is_empty() {
        return Ok(default);
    }
    serde_json::from_str(&text).map_err(|e| format!("解析 {file} 失败: {e}"))
}

fn write_json(app: &AppHandle, file: &str, value: &Value) -> Result<(), String> {
    let path = config_dir(app)?.join(file);
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| format!("序列化 {file} 失败: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("写入 {file} 失败: {e}"))
}

#[tauri::command]
pub fn load_settings(app: AppHandle) -> Result<Value, String> {
    read_json(&app, "config.json", default_settings())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Value) -> Result<(), String> {
    write_json(&app, "config.json", &settings)
}

#[tauri::command]
pub fn load_items(app: AppHandle) -> Result<Value, String> {
    read_json(
        &app,
        "items.json",
        serde_json::json!({
            "launcherItems": [],
            "resources": []
        }),
    )
}

#[tauri::command]
pub fn save_items(app: AppHandle, items: Value) -> Result<(), String> {
    write_json(&app, "items.json", &items)
}

fn default_settings() -> Value {
    serde_json::json!({
        "version": 1,
        "theme": "aurora",
        "accent": "#5b8cff",
        "ports": {
            "refreshMs": 5000,
            "includeSystem": false
        },
        "monitor": {
            "refreshMs": 1000,
            "historyLen": 60
        },
        "launcher": {
            "autoScan": true,
            "extraPaths": []
        }
    })
}
