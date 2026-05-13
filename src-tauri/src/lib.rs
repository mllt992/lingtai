mod commands;
mod state;

use state::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

/// 安装全局 panic hook，吞掉 `lnk` crate 内部的 panic（库代码会在
/// 解析某些合法 .lnk 时直接 unwrap None）。我们在 launcher::resolve
/// 处用 catch_unwind 兜住，这里只是把 stderr 上的噪声日志消掉。
fn install_panic_filter() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(loc) = info.location() {
            if loc.file().contains("lnk-0.5") {
                return;
            }
        }
        prev(info);
    }));
}

/// 把主窗口恢复到前台 —— 兼容隐藏 / 最小化 / 失焦等多种状态。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_filter();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .setup(|app| {
            // ===== 系统托盘 =====
            // 菜单：显示主窗口 / ─── / 退出
            let show_item = MenuItem::with_id(app, "tray-show", "显示主窗口", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "tray-quit", "退出 Loft", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("window icon should be configured in tauri.conf.json");

            TrayIconBuilder::with_id("loft-tray")
                .tooltip("凌台 · Loft（点击显示主窗口）")
                .icon(icon)
                .menu(&menu)
                // 左键自定义处理（默认行为是弹菜单，我们改成"切换窗口可见性"）
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "tray-show" => show_main_window(app),
                    "tray-quit" => {
                        // 真退出：跳过窗口 CloseRequested 拦截
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // 单击左键 → 切换主窗口可见性
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let visible = window.is_visible().unwrap_or(false);
                            let focused = window.is_focused().unwrap_or(false);
                            if visible && focused {
                                let _ = window.hide();
                            } else {
                                show_main_window(app);
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        // 主窗口关闭事件 → 拦截为"隐藏到托盘"。只有托盘菜单的"退出"才会真正退出。
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::launcher::scan_start_menu,
            commands::launcher::resolve_shortcut,
            commands::launcher::launch_path,
            commands::files::open_path,
            commands::files::reveal_in_explorer,
            commands::files::open_url,
            commands::icons::extract_icon,
            commands::monitor::get_system_snapshot,
            commands::monitor::list_drives,
            commands::ports::list_user_ports,
            commands::ports::kill_process,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::settings::load_items,
            commands::settings::save_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Loft");
}
