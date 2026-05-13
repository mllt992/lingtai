mod commands;
mod state;

use state::AppState;

/// Install a global panic hook that silently swallows panics originating from
/// the `lnk` crate. lnk 0.5.1 panics (instead of returning Err) on some valid
/// Windows shortcuts — we catch them at the call site, this hook just hides
/// the noisy backtrace from stderr.
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_filter();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::launcher::scan_start_menu,
            commands::launcher::resolve_shortcut,
            commands::launcher::launch_path,
            commands::files::open_path,
            commands::files::reveal_in_explorer,
            commands::files::open_url,
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
