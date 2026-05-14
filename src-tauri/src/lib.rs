mod commands;
mod state;
mod tray;

use state::AppState;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};

const TRAY_MAIN: &str = "loft-main";

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

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_hud(app: &tauri::AppHandle) {
    if let Some(hud) = app.get_webview_window("hud") {
        let visible = hud.is_visible().unwrap_or(false);
        if visible {
            let _ = hud.hide();
        } else {
            let _ = hud.show();
        }
    }
}

/// 后台线程：每 2 秒采集指标，给 HUD 发 event，给主托盘更新图标 + tooltip
fn spawn_metrics_loop(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        // 首次拉取地理信息（失败也无所谓，下次再试）
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tray::maybe_refresh_geo();
        }));

        loop {
            // 整次采集 + 推送都用 catch_unwind 包起来 ——
            // 任何单次错误（系统调用失败、emit 失败等）都不会击穿后台线程。
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_metrics_tick(&app_handle);
            }));
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

fn run_metrics_tick(app_handle: &tauri::AppHandle) {
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

    // 1) 主托盘图标 + tooltip
    if let Some(tray_icon) = app_handle.tray_by_id(TRAY_MAIN) {
        let base = tray::base_icon();
        let rgba = tray::render_icon_with_cpu(cpu_pct);
        let image = Image::new_owned(rgba, base.width, base.height);
        let _ = tray_icon.set_icon(Some(image));
        let _ = tray_icon.set_tooltip(Some(tray::format_main_tooltip(
            cpu_pct,
            mem_pct,
            temp,
            geo.as_ref(),
        )));
    }

    // 2) emit "metrics" 给 HUD
    let payload = tray::build_metrics(cpu_pct, mem_pct, temp, geo.as_ref());
    let _ = app_handle.emit("metrics", payload);
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
            // ===== 系统托盘 (单个 LOGO 图标 + 菜单) =====
            let show_item =
                MenuItem::with_id(app, "tray-show", "显示主窗口", true, None::<&str>)?;
            let hud_item =
                MenuItem::with_id(app, "tray-hud", "切换数据面板", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item =
                MenuItem::with_id(app, "tray-quit", "退出 Loft", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &hud_item, &sep, &quit_item])?;

            let main_icon = app
                .default_window_icon()
                .cloned()
                .expect("window icon should be configured in tauri.conf.json");

            TrayIconBuilder::with_id(TRAY_MAIN)
                .tooltip("凌台 · Loft")
                .icon(main_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "tray-show" => show_main_window(app),
                    "tray-hud" => toggle_hud(app),
                    "tray-quit" => app.exit(0),
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

            // HUD 窗口完全交给前端管理（visible: true 已在配置里），
            // 在这里不做任何 show / set_position / set_always_on_top —— 任何 setup 阶段
            // 的 HUD API 失败都可能拖垮整个 app 启动。前端 onMounted 自己定位。

            // ===== 后台数据循环（更新托盘 + emit metrics 给 HUD） =====
            spawn_metrics_loop(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // main 关闭 → 隐藏到托盘；hud 关闭 → 真隐藏
                if window.label() == "main" || window.label() == "hud" {
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
