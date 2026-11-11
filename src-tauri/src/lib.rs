pub mod config;
pub mod ddc;
pub mod hotkeys;
pub mod local_port;
pub mod model;
pub mod monitor_service;
pub mod platform;
pub mod toggle;
pub mod view;

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running MonitorSwitch");
}
