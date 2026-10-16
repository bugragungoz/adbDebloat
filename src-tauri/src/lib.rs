//! The adbDebloat window and the commands it calls. Everything about adb and Android lives in
//! `adb-core`; this crate wires it to Tauri.

mod commands;
mod settings;
mod state;

use std::sync::Arc;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Two windows would send commands to the same phone at once; show the first instead.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir().ok();
            let data_dir = app.path().app_local_data_dir()?;
            let shared = Arc::new(state::AppState::new(config_dir, data_dir));
            if shared.community_is_due() {
                let background = Arc::clone(&shared);
                std::thread::spawn(move || {
                    // Offline is fine: the built-in list (or the last download) stays in use.
                    let _ = background.refresh_community();
                });
            }
            app.manage(shared);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_device,
            commands::get_packages,
            commands::apply,
            commands::stop_batch,
            commands::get_history,
            commands::pair,
            commands::connect,
            commands::disconnect,
            commands::switch_to_wifi,
            commands::reboot,
            commands::download_platform_tools,
            commands::refresh_knowledge,
            commands::get_settings,
            commands::set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("adbDebloat could not start");
}
