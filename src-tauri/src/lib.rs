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
pub mod updates;
pub mod view;

use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Modifiers, ShortcutState};

const QUIT_ARG: &str = "--quit";
const SETTINGS_ARG: &str = "--settings";

pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin: a second launch just opens the running app's panel,
        // opens its settings with --settings, or quits it with --quit (used before
        // reinstalling, so no stale tray icon is left).
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|a| a == QUIT_ARG) {
                app.exit(0);
            } else if args.iter().any(|a| a == SETTINGS_ARG) {
                // Creating a window here would deadlock on Windows; see app::windows.
                let app = app.clone();
                std::thread::spawn(move || app::windows::open_settings(&app));
            } else {
                app::flyout::show_default(app);
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::<tauri::Wry>::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if shortcut.mods.intersects(Modifiers::ALT | Modifiers::SUPER) {
                            platform::mask_modifier_release();
                        }
                        app::actions::on_hotkey(app, shortcut.id());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::refresh,
            commands::switch_input,
            commands::toggle,
            commands::set_port_name,
            commands::set_port_hotkey,
            commands::set_toggle_hotkey,
            commands::start_hotkey_recording,
            commands::stop_hotkey_recording,
            commands::set_toggle_pair,
            commands::set_local_override,
            commands::set_read_capabilities,
            commands::set_option,
            commands::open_settings,
            commands::hide_flyout,
            commands::fit_flyout,
            commands::check_for_updates,
            commands::install_update,
            commands::open_release_page,
        ])
        .setup(|app| {
            // Only reached when no other copy is running (the single-instance plugin forwards
            // the arguments to a running copy and ends this process first): nothing to quit.
            if std::env::args().any(|a| a == QUIT_ARG) {
                std::process::exit(0);
            }
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
