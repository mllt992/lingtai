// Prevent additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebView2 默认数据目录有时不可写/损坏，会导致白屏。固定到 AppData 下。
    if std::env::var_os("WEBVIEW2_USER_DATA_FOLDER").is_none() {
        if let Ok(base) = std::env::var("LOCALAPPDATA") {
            let dir = std::path::Path::new(&base).join("com.loft.app").join("WebView2");
            let _ = std::fs::create_dir_all(&dir);
            std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &dir);
        }
    }
    loft_lib::run();
}
