//! Everything the UI shows, computed from monitor state and settings.

use serde::Serialize;

use crate::config::Config;
use crate::ddc::caps::input_label;
use crate::model::{ConnectionKind, MonitorKey};
use crate::monitor_service::MonitorState;
use crate::toggle::default_pair;
use crate::updates::UpdateStatus;

pub const UNNAMED: &str = "未命名";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortView {
    pub value: u16,
    /// "DP", "HDMI", "USB-C"…
    pub port_label: String,
    /// The name the user gave it.
    pub name: Option<String>,
    /// What to show: the name, this computer's name, or "未命名".
    pub display_name: String,
    pub is_local: bool,
    pub is_current: bool,
    pub hotkey: Option<String>,
    /// "desktop" | "laptop" | "plug"
    pub icon: &'static str,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorView {
    pub key: MonitorKey,
    pub label: String,
    pub connection: ConnectionKind,
    pub current: Option<u16>,
    pub current_name: Option<String>,
    pub error: Option<String>,
    pub switch_error: Option<String>,
    pub ports: Vec<PortView>,
    pub toggle_pair: Option<[u16; 2]>,
    pub toggle_pair_is_default: bool,
    pub local_override: Option<u16>,
    pub read_capabilities: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub monitors: Vec<MonitorView>,
    pub show_osd: bool,
    pub autostart: bool,
    pub lan_discovery: bool,
    pub toggle_hotkey: Option<String>,
    pub computer_name: String,
    /// Accelerators the OS refused to register (filled in by AppState).
    pub failed_hotkeys: Vec<String>,
    pub check_updates: bool,
    /// This build's version (filled in by AppState).
    pub app_version: String,
    /// What the update reminder shows (filled in by AppState).
    pub update: UpdateStatus,
}

pub fn build_view(states: &[MonitorState], cfg: &Config, is_laptop: bool, computer_name: &str) -> View {
    View {
        monitors: states.iter().map(|s| monitor_view(s, cfg, is_laptop, computer_name)).collect(),
        show_osd: cfg.show_osd,
        autostart: cfg.autostart,
        lan_discovery: cfg.lan_discovery,
        toggle_hotkey: cfg.toggle_hotkey.clone(),
        computer_name: computer_name.to_string(),
        failed_hotkeys: Vec::new(),
        check_updates: cfg.check_updates,
        app_version: String::new(),
        update: UpdateStatus::Idle,
    }
}

fn monitor_view(s: &MonitorState, cfg: &Config, is_laptop: bool, computer_name: &str) -> MonitorView {
    let mc = cfg.monitor(&s.key);

    let mut values = s.inputs.clone();
    if let Some(mc) = mc {
        values.extend(mc.ports.keys().filter_map(|k| k.parse::<u16>().ok()));
    }
    values.extend(s.current);
    values.extend(s.local_input);
    values.sort();
    values.dedup();

    let mut ports: Vec<PortView> = values
        .into_iter()
        .map(|value| {
            let pc = mc.and_then(|m| m.port(value));
            let name = pc.and_then(|p| p.name.clone());
            let is_local = s.local_input == Some(value);
            let display_name = match (&name, is_local) {
                (Some(n), _) => n.clone(),
                (None, true) => computer_name.to_string(),
                (None, false) => UNNAMED.to_string(),
            };
            // With two computers on one monitor, the other one is usually the other kind.
            let (this_kind, other_kind) = if is_laptop { ("laptop", "desktop") } else { ("desktop", "laptop") };
            let icon = if is_local {
                this_kind
            } else if name.is_some() {
                other_kind
            } else {
                "plug"
            };
            PortView {
                value,
                port_label: input_label(value),
                name,
                display_name,
                is_local,
                is_current: s.current == Some(value),
                hotkey: pc.and_then(|p| p.hotkey.clone()),
                icon,
            }
        })
        .collect();
    ports.sort_by_key(|p| (!p.is_local, p.name.is_none(), p.value));

    let named: Vec<u16> = ports.iter().filter(|p| p.name.is_some()).map(|p| p.value).collect();
    let configured_pair = mc.and_then(|m| m.toggle_pair);
    let toggle_pair = configured_pair.or_else(|| default_pair(s.local_input, &named, &s.inputs));
    let current_name = s
        .current
        .and_then(|c| ports.iter().find(|p| p.value == c))
        .map(|p| p.display_name.clone());

    MonitorView {
        key: s.key.clone(),
        label: s.label.clone(),
        connection: s.connection,
        current: s.current,
        current_name,
        error: s.error.clone(),
        switch_error: s.switch_error.clone(),
        ports,
        toggle_pair,
        toggle_pair_is_default: configured_pair.is_none(),
        local_override: mc.and_then(|m| m.local_port_override),
        read_capabilities: mc.map(|m| m.read_capabilities).unwrap_or(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> MonitorKey {
        MonitorKey("DEL426A#UID1".into())
    }

    fn state(current: Option<u16>, local: Option<u16>) -> MonitorState {
        MonitorState {
            key: key(),
            label: "Dell U2723QE".into(),
            connection: ConnectionKind::DisplayPort,
            inputs: vec![0x0F, 0x11, 0x1B],
            current,
            local_input: local,
            error: None,
            switch_error: None,
        }
    }

    fn values(view: &View) -> Vec<u16> {
        view.monitors[0].ports.iter().map(|p| p.value).collect()
    }

    #[test]
    fn local_port_comes_first_and_shows_the_computer_name() {
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &Config::default(), false, "DESKTOP-1");
        let port = &view.monitors[0].ports[0];
        assert_eq!(port.value, 0x0F);
        assert!(port.is_local && port.is_current);
        assert_eq!(port.display_name, "DESKTOP-1");
        assert_eq!(port.port_label, "DP");
        assert_eq!(port.icon, "desktop");
        assert_eq!(view.monitors[0].current_name.as_deref(), Some("DESKTOP-1"));
    }

    #[test]
    fn named_ports_come_before_unnamed_ones() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x1B).name = Some("公司笔记本".into());
        let view = build_view(&[state(Some(0x1B), Some(0x0F))], &cfg, false, "DESKTOP-1");
        assert_eq!(values(&view), vec![0x0F, 0x1B, 0x11]);
        let ports = &view.monitors[0].ports;
        assert_eq!(ports[1].display_name, "公司笔记本");
        assert_eq!(ports[1].icon, "laptop");
        assert!(ports[1].is_current);
        assert_eq!(ports[2].display_name, UNNAMED);
        assert_eq!(ports[2].icon, "plug");
        assert_eq!(view.monitors[0].current_name.as_deref(), Some("公司笔记本"));
    }

    #[test]
    fn configured_ports_the_monitor_does_not_list_are_shown() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x12).name = Some("游戏机".into());
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert!(values(&view).contains(&0x12));
    }

    #[test]
    fn default_toggle_pair_is_local_plus_first_named() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x11).name = Some("游戏机".into());
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert_eq!(view.monitors[0].toggle_pair, Some([0x0F, 0x11]));
        assert!(view.monitors[0].toggle_pair_is_default);
    }

    #[test]
    fn configured_toggle_pair_wins() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).toggle_pair = Some([0x11, 0x0F]);
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert_eq!(view.monitors[0].toggle_pair, Some([0x11, 0x0F]));
        assert!(!view.monitors[0].toggle_pair_is_default);
    }

    #[test]
    fn on_a_laptop_the_icons_swap() {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x0F).name = Some("主机".into());
        let view = build_view(&[state(Some(0x1B), Some(0x1B))], &cfg, true, "LAPTOP");
        let ports = &view.monitors[0].ports;
        assert_eq!((ports[0].value, ports[0].icon), (0x1B, "laptop"));
        assert_eq!((ports[1].value, ports[1].icon), (0x0F, "desktop"));
    }

    #[test]
    fn copies_options_and_hotkeys() {
        let mut cfg = Config::default();
        cfg.show_osd = false;
        cfg.monitor_mut(&key()).port_mut(0x1B).hotkey = Some("Ctrl+Alt+2".into());
        let view = build_view(&[state(Some(0x0F), Some(0x0F))], &cfg, false, "PC");
        assert!(!view.show_osd);
        assert_eq!(view.toggle_hotkey.as_deref(), Some("Ctrl+Alt+M"));
        let usb_c = view.monitors[0].ports.iter().find(|p| p.value == 0x1B).unwrap();
        assert_eq!(usb_c.hotkey.as_deref(), Some("Ctrl+Alt+2"));
    }
}
