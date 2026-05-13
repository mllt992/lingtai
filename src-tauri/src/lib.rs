mod commands;
mod state;
mod tray;

use state::AppState;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

const TRAY_TEXT_SIZE: u32 = 32;

const TRAY_MAIN: &str = "loft-main";
const TRAY_CPU: &str = "loft-cpu";
const TRAY_TEMP: &str = "loft-temp";
const TRAY_GEO: &str = "loft-geo";

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

/// 后台线程：每 2 秒刷新所有托盘图标的内容（图像 + tooltip）。
/// 公网 IP / 国家 5 分钟才再请求一次（失败 30 秒后重试）。
fn spawn_tray_updater(app_handle: tauri::AppHandle) {
    std::thread::spawn(move || {
        // 启动后先打一次（不阻塞主线程，结果异步可用）
        tray::maybe_refresh_geo();

        loop {
            // ===== 读取所有指标 =====
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

            // ===== 主图标：LOGO + CPU 进度条 + 综合 tooltip =====
            if let Some(tray) = app_handle.tray_by_id(TRAY_MAIN) {
                let base = tray::base_icon();
                let rgba = tray::render_icon_with_cpu(cpu_pct);
                let image = Image::new_owned(rgba, base.width, base.height);
                let _ = tray.set_icon(Some(image));
                let _ = tray.set_tooltip(Some(tray::format_main_tooltip(
                    cpu_pct,
                    mem_pct,
                    temp,
                    geo.as_ref(),
                )));
            }

            // ===== CPU 数字图标 =====
            if let Some(tray) = app_handle.tray_by_id(TRAY_CPU) {
                let txt = tray::cpu_text(cpu_pct);
                let bg = tray::cpu_bg(cpu_pct);
                let rgba = tray::render_text_icon(&txt, TRAY_TEXT_SIZE, bg);
                let image = Image::new_owned(rgba, TRAY_TEXT_SIZE, TRAY_TEXT_SIZE);
                let _ = tray.set_icon(Some(image));
                let _ = tray.set_tooltip(Some(format!("CPU {:.1}%  内存 {:.1}%", cpu_pct, mem_pct)));
            }

            // ===== 温度图标：拿不到就隐藏 =====
            if let Some(tray) = app_handle.tray_by_id(TRAY_TEMP) {
                match temp {
                    Some(t) => {
                        let txt = tray::temp_text(t);
                        let bg = tray::temp_bg(t);
                        let rgba = tray::render_text_icon(&txt, TRAY_TEXT_SIZE, bg);
                        let image = Image::new_owned(rgba, TRAY_TEXT_SIZE, TRAY_TEXT_SIZE);
                        let _ = tray.set_icon(Some(image));
                        let _ = tray.set_tooltip(Some(format!("CPU 温度 {:.0} °C", t)));
                        let _ = tray.set_visible(true);
                    }
                    None => {
                        let _ = tray.set_visible(false);
                    }
                }
            }

            // ===== 国家图标：拿不到就隐藏 =====
            if let Some(tray) = app_handle.tray_by_id(TRAY_GEO) {
                match geo.as_ref() {
                    Some(g) => {
                        let bg = tray::geo_bg();
                        let rgba = tray::render_text_icon(&g.country, TRAY_TEXT_SIZE, bg);
                        let image = Image::new_owned(rgba, TRAY_TEXT_SIZE, TRAY_TEXT_SIZE);
                        let _ = tray.set_icon(Some(image));
                        let mut tip = format!("{} · {}", g.country, g.ip);
                        if let Some(city) = &g.city {
                            tip.push_str(&format!(" · {}", city));
                        }
                        let _ = tray.set_tooltip(Some(tip));
                        let _ = tray.set_visible(true);
                    }
                    None => {
                        let _ = tray.set_visible(false);
                    }
                }
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
            // ===== 主托盘 (LOGO + 菜单) =====
            let show_item = MenuItem::with_id(app, "tray-show", "显示主窗口", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "tray-quit", "退出 Loft", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;
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

            // ===== 文字数据图标（CPU / 温度 / 国家） =====
            // 占位图标：先用一个 "--" 图，后台线程立刻会覆盖
            let placeholder_cpu =
                tray::render_text_icon("--", TRAY_TEXT_SIZE, (0x10, 0xb9, 0x81));
            let placeholder_temp =
                tray::render_text_icon("--", TRAY_TEXT_SIZE, (0xf9, 0x73, 0x16));
            let placeholder_geo =
                tray::render_text_icon("--", TRAY_TEXT_SIZE, tray::geo_bg());

            TrayIconBuilder::with_id(TRAY_CPU)
                .tooltip("CPU 占用")
                .icon(Image::new_owned(
                    placeholder_cpu,
                    TRAY_TEXT_SIZE,
                    TRAY_TEXT_SIZE,
                ))
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| open_window_on_click(tray, &event))
                .build(app)?;

            TrayIconBuilder::with_id(TRAY_TEMP)
                .tooltip("CPU 温度")
                .icon(Image::new_owned(
                    placeholder_temp,
                    TRAY_TEXT_SIZE,
                    TRAY_TEXT_SIZE,
                ))
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| open_window_on_click(tray, &event))
                .build(app)?;
            // 温度默认隐藏，拿到读数才显示
            if let Some(t) = app.tray_by_id(TRAY_TEMP) {
                let _ = t.set_visible(false);
            }

            TrayIconBuilder::with_id(TRAY_GEO)
                .tooltip("公网 IP / 国家")
                .icon(Image::new_owned(
                    placeholder_geo,
                    TRAY_TEXT_SIZE,
                    TRAY_TEXT_SIZE,
                ))
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| open_window_on_click(tray, &event))
                .build(app)?;
            if let Some(t) = app.tray_by_id(TRAY_GEO) {
                let _ = t.set_visible(false);
            }

            // 后台更新器
            spawn_tray_updater(app.handle().clone());

            Ok(())
        })
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

/// 任何托盘图标左键点击都打开/恢复主窗口
fn open_window_on_click(
    tray: &tauri::tray::TrayIcon,
    event: &tauri::tray::TrayIconEvent,
) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let app = tray.app_handle();
        show_main_window(app);
    }
}
