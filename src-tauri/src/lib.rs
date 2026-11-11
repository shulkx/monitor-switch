mod app;
mod commands;
pub mod config;
pub mod ddc;
pub mod hotkeys;
pub mod local_port;
pub mod model;
pub mod monitor_service;
pub mod platform;
pub mod toggle;
pub mod view;

use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin: a second launch just opens the running app's panel.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            app::flyout::show_default(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::<tauri::Wry>::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        app::actions::on_hotkey(app, shortcut.id());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::refresh,
            commands::switch_input,
            commands::toggle,
            commands::set_port_name,
            commands::set_port_hotkey,
            commands::set_toggle_hotkey,
            commands::pause_hotkeys,
            commands::resume_hotkeys,
            commands::set_toggle_pair,
            commands::set_local_override,
            commands::set_read_capabilities,
            commands::set_option,
            commands::open_settings,
            commands::hide_flyout,
            commands::fit_flyout,
        ])
        .setup(|app| {
            app::setup(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building MonitorSwitch")
        .run(|_app, event| {
            // Closing the settings window must not quit the tray app; "退出" calls exit(0).
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
