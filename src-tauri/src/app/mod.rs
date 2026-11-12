pub mod actions;
pub mod flyout;
pub mod osd;
pub mod state;
pub mod tray;
pub mod updater;
pub mod windows;

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};

use crate::config::ConfigStore;
use crate::hotkeys::HotkeyRegistry;
use crate::monitor_service::{MonitorService, Timing};
use crate::platform::{self, NativeBackend};
use flyout::FlyoutState;
use state::{AppState, ConfigPrefs};

pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let store = ConfigStore::new(app.path().config_dir()?.join("MonitorSwitch").join("config.json"));
    let mut cfg = store.load();
    // A capabilities read that never finished crashed us last time (spec 6.4).
    if !cfg.recover_interrupted_caps_reads().is_empty() {
        let _ = store.save(&cfg);
    }
    let config = Arc::new(Mutex::new(cfg));
    let prefs = Arc::new(ConfigPrefs { config: config.clone(), store: store.clone() });
    let is_laptop = platform::is_laptop();
    let handle = app.clone();
    let service = MonitorService::start(
        Box::new(NativeBackend::new()),
        prefs,
        is_laptop,
        Timing::production(),
        move |event| actions::handle_service_event(&handle, event),
    );
    app.manage(AppState {
        config,
        store,
        service,
        hotkeys: HotkeyRegistry::default(),
        failed_hotkeys: Mutex::new(Vec::new()),
        last_view: Mutex::new(None),
        flyout: FlyoutState::default(),
        is_laptop,
        computer_name: platform::computer_name(),
        app_version: app.package_info().version.to_string(),
        updates: updater::UpdateState::default(),
    });

    windows::create_flyout(app)?;
    windows::create_osd(app)?;
    tray::create(app)?;
    actions::apply_hotkeys(app);
    actions::sync_autostart(app);
    actions::publish_view(app);
    updater::start_background_checks(app);
    Ok(())
}
