mod commands;
mod state;
mod tray;

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

/// 在后台线程里每 2 秒刷新一次托盘的 tooltip 和图标（CPU 条）。
/// 公网 IP / 国家 5 分钟才再请求一次，避免频繁外网调用。
fn spawn_tray_updater(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        // 进程启动时先尝试拿一次地理位置（不阻塞，失败就稍后再试）
        tray::maybe_refresh_geo();

        loop {
            // 1) 读 CPU / 内存（复用 AppState 里的 sysinfo 句柄）
            let (cpu_pct, mem_pct) = {
                let state = app_handle.state::<AppState>();
                let mut sys = state.sys.lock();
                sys.refresh_cpu_usage();
                sys.refresh_memory();
                let cpu = sys.global_cpu_usage();
                let mem = if sys.total_memory() > 0 {
                    (sys.used_memory() as f32 / sys.total_memory() as f32) * 100.0
                } else {
                    0.0
                };
                (cpu, mem)
            };
            let temp = tray::read_cpu_temp();
            tray::maybe_refresh_geo();
            let geo = tray::current_geo();

            // 2) 更新托盘
            if let Some(tray_icon) = app_handle.tray_by_id("loft-tray") {
                // tooltip：多行
                let tip = tray::format_tooltip(cpu_pct, mem_pct, temp, geo.as_ref());
                let _ = tray_icon.set_tooltip(Some(tip));

                // 图标：基础 LOGO + 底部 CPU 进度条
                let base = tray::base_icon();
                let rgba = tray::render_icon_with_cpu(cpu_pct);
                let image = tauri::image::Image::new_owned(rgba, base.width, base.height);
                let _ = tray_icon.set_icon(Some(image));
            }

            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
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
            let show_item = MenuItem::with_id(app, "tray-show", "显示主窗口", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "tray-quit", "退出 Loft", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("window icon should be configured in tauri.conf.json");

            TrayIconBuilder::with_id("loft-tray")
                .tooltip("凌台 · Loft")
                .icon(icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "tray-show" => show_main_window(app),
                    "tray-quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
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

            // ===== 后台更新器：CPU 条 + 多行 tooltip =====
            spawn_tray_updater(app.handle().clone());

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
