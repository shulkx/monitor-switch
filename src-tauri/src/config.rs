//! Settings file (spec 5.4).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::MonitorKey;

pub const CONFIG_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub version: u32,
    pub show_osd: bool,
    /// Used by plan 3 (LAN discovery).
    pub lan_discovery: bool,
    pub autostart: bool,
    pub toggle_hotkey: Option<String>,
    pub monitors: BTreeMap<String, MonitorConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: CONFIG_VERSION,
            show_osd: true,
            lan_discovery: true,
            autostart: true,
            toggle_hotkey: Some("Ctrl+Alt+M".into()),
            monitors: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MonitorConfig {
    pub label: String,
    pub read_capabilities: bool,
    /// True while a capabilities read is in progress. Still true at startup means that
    /// read crashed the app (or Windows), see spec 6.4.
    pub caps_read_pending: bool,
    pub cached_capabilities: Option<String>,
    pub local_port_override: Option<u16>,
    pub toggle_pair: Option<[u16; 2]>,
    /// Keyed by the input value in decimal, because JSON object keys are strings.
    pub ports: BTreeMap<String, PortConfig>,
    /// Used by plan 2 (learn ports).
    pub learned_ports: BTreeMap<String, String>,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        MonitorConfig {
            label: String::new(),
            read_capabilities: true,
            caps_read_pending: false,
            cached_capabilities: None,
            local_port_override: None,
            toggle_pair: None,
            ports: BTreeMap::new(),
            learned_ports: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PortConfig {
    pub name: Option<String>,
    pub hotkey: Option<String>,
    /// Used by plan 2 (new device prompt).
    pub ignored: bool,
}

impl Config {
    pub fn monitor(&self, key: &MonitorKey) -> Option<&MonitorConfig> {
        self.monitors.get(&key.0)
    }

    pub fn monitor_mut(&mut self, key: &MonitorKey) -> &mut MonitorConfig {
        self.monitors.entry(key.0.clone()).or_default()
    }

    /// Turn off automatic capability reads for monitors whose last read never finished,
    /// and return their labels.
    pub fn recover_interrupted_caps_reads(&mut self) -> Vec<String> {
        let mut labels = Vec::new();
        for m in self.monitors.values_mut() {
            if m.caps_read_pending {
                m.caps_read_pending = false;
                m.read_capabilities = false;
                labels.push(m.label.clone());
            }
        }
        labels
    }
}

impl MonitorConfig {
    pub fn port(&self, value: u16) -> Option<&PortConfig> {
        self.ports.get(&value.to_string())
    }

    pub fn port_mut(&mut self, value: u16) -> &mut PortConfig {
        self.ports.entry(value.to_string()).or_default()
    }

    /// Input values that have a name, ascending.
    pub fn named_ports(&self) -> Vec<u16> {
        let mut values: Vec<u16> = self
            .ports
            .iter()
            .filter(|(_, p)| p.name.is_some())
            .filter_map(|(k, _)| k.parse().ok())
            .collect();
        values.sort();
        values
    }
}

#[derive(Clone, Debug)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: PathBuf) -> Self {
        ConfigStore { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Missing file → defaults. Corrupt file → moved to `config.bad.json`, defaults returned.
    pub fn load(&self) -> Config {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return Config::default();
        };
        match serde_json::from_str::<Config>(&text) {
            Ok(cfg) => cfg,
            Err(_) => {
                let _ = fs::rename(&self.path, self.path.with_file_name("config.bad.json"));
                Config::default()
            }
        }
    }

    /// Write a temporary file, then rename it over the real one, so a crash never leaves half a file.
    pub fn save(&self, cfg: &Config) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(cfg).map_err(io::Error::other)?;
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, ConfigStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = ConfigStore::new(dir.path().join("MonitorSwitch").join("config.json"));
        (dir, store)
    }

    #[test]
    fn defaults_match_the_spec() {
        let cfg = Config::default();
        assert!(cfg.show_osd && cfg.autostart && cfg.lan_discovery);
        assert_eq!(cfg.toggle_hotkey.as_deref(), Some("Ctrl+Alt+M"));
        assert!(MonitorConfig::default().read_capabilities);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let (_dir, store) = store();
        assert_eq!(store.load(), Config::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let (_dir, store) = store();
        let mut cfg = Config::default();
        cfg.show_osd = false;
        let key = MonitorKey("DEL426A#UID1".into());
        cfg.monitor_mut(&key).port_mut(0x1B).name = Some("公司笔记本".into());
        cfg.monitor_mut(&key).toggle_pair = Some([0x0F, 0x1B]);
        store.save(&cfg).unwrap();
        assert_eq!(store.load(), cfg);
    }

    #[test]
    fn partial_file_fills_missing_fields_with_defaults() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), r#"{"showOsd": false}"#).unwrap();
        let cfg = store.load();
        assert!(!cfg.show_osd);
        assert!(cfg.autostart);
    }

    #[test]
    fn corrupt_file_is_moved_aside_and_defaults_are_used() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "{ not json").unwrap();
        assert_eq!(store.load(), Config::default());
        assert!(store.path().with_file_name("config.bad.json").exists());
        assert!(!store.path().exists());
    }

    #[test]
    fn recovers_interrupted_capability_reads() {
        let mut cfg = Config::default();
        let key = MonitorKey("BAD#1".into());
        let m = cfg.monitor_mut(&key);
        m.label = "Crashy 27".into();
        m.caps_read_pending = true;
        assert_eq!(cfg.recover_interrupted_caps_reads(), vec!["Crashy 27".to_string()]);
        let m = cfg.monitor(&key).unwrap();
        assert!(!m.caps_read_pending);
        assert!(!m.read_capabilities);
    }

    #[test]
    fn named_ports_lists_only_named_inputs_in_order() {
        let mut m = MonitorConfig::default();
        m.port_mut(0x1B).name = Some("笔记本".into());
        m.port_mut(0x11).hotkey = Some("Ctrl+Alt+2".into());
        m.port_mut(0x0F).name = Some("主机".into());
        assert_eq!(m.named_ports(), vec![0x0F, 0x1B]);
    }
}
