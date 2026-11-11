use std::sync::{Arc, Mutex};

use crate::app::flyout::FlyoutState;
use crate::config::{Config, ConfigStore};
use crate::hotkeys::HotkeyRegistry;
use crate::model::MonitorKey;
use crate::monitor_service::{CapsDecision, MonitorPrefs, MonitorService};
use crate::view::{build_view, View};

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub store: ConfigStore,
    pub service: MonitorService,
    pub hotkeys: HotkeyRegistry,
    pub failed_hotkeys: Mutex<Vec<String>>,
    pub flyout: FlyoutState,
    pub is_laptop: bool,
    pub computer_name: String,
}

impl AppState {
    pub fn view(&self) -> View {
        let cfg = self.config.lock().unwrap();
        let mut view = build_view(&self.service.snapshot(), &cfg, self.is_laptop, &self.computer_name);
        view.failed_hotkeys = self.failed_hotkeys.lock().unwrap().clone();
        view
    }

    /// Change the settings and save them.
    pub fn update<F: FnOnce(&mut Config)>(&self, change: F) -> Result<(), String> {
        let mut cfg = self.config.lock().unwrap();
        change(&mut cfg);
        self.store.save(&cfg).map_err(|e| format!("保存设置失败：{e}"))
    }
}

/// MonitorService's view of the settings file.
pub struct ConfigPrefs {
    pub config: Arc<Mutex<Config>>,
    pub store: ConfigStore,
}

impl MonitorPrefs for ConfigPrefs {
    fn caps_decision(&self, key: &MonitorKey) -> CapsDecision {
        let cfg = self.config.lock().unwrap();
        match cfg.monitor(key) {
            Some(m) if !m.read_capabilities => CapsDecision::Skip,
            Some(m) => match &m.cached_capabilities {
                Some(raw) => CapsDecision::Cached(raw.clone()),
                None => CapsDecision::Read,
            },
            None => CapsDecision::Read,
        }
    }

    fn begin_caps_read(&self, key: &MonitorKey, label: &str) {
        let mut cfg = self.config.lock().unwrap();
        let m = cfg.monitor_mut(key);
        m.label = label.to_string();
        m.caps_read_pending = true;
        let _ = self.store.save(&cfg);
    }

    fn finish_caps_read(&self, key: &MonitorKey, raw: Option<String>) {
        let mut cfg = self.config.lock().unwrap();
        let m = cfg.monitor_mut(key);
        m.caps_read_pending = false;
        // An empty string still counts as "read", so a monitor that never answers isn't asked forever.
        m.cached_capabilities = Some(raw.unwrap_or_default());
        let _ = self.store.save(&cfg);
    }

    fn local_override(&self, key: &MonitorKey) -> Option<u16> {
        self.config.lock().unwrap().monitor(key).and_then(|m| m.local_port_override)
    }
}
