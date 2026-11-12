//! Global hotkeys: which accelerator does what, conflicts, and registration with the OS.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

use crate::config::Config;
use crate::model::MonitorKey;

pub const BAD_FORMAT: &str = "这个组合键用不了，换一个";
pub const IN_USE_HERE: &str = "这个快捷键已经给别的接口用了";
pub const IN_USE_ELSEWHERE: &str = "已被其他程序占用，换一个";
pub const TOO_COMMON: &str = "只带一个 Ctrl、Alt 或 Shift 的组合会抢走其他软件的快捷键（比如 Ctrl+C），再加一个，比如 Ctrl+Alt+…";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleAll,
    SwitchTo { key: MonitorKey, value: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeyTarget {
    Toggle,
    Port { key: MonitorKey, value: u16 },
}

/// The OS-level id of an accelerator such as "Ctrl+Alt+M" (case-insensitive).
pub fn shortcut_id(accelerator: &str) -> Result<u32, String> {
    Shortcut::from_str(accelerator.trim())
        .map(|s| s.id())
        .map_err(|_| BAD_FORMAT.to_string())
}

/// Can `accelerator` be a global hotkey? A global hotkey takes the combination away from every
/// other program, so one with a single Ctrl, Alt or Shift (Ctrl+C, Alt+F4, Shift+A) is refused.
pub fn check_combination(accelerator: &str) -> Result<(), String> {
    let shortcut = Shortcut::from_str(accelerator.trim()).map_err(|_| BAD_FORMAT.to_string())?;
    let mods = shortcut.mods;
    let count = [Modifiers::CONTROL, Modifiers::ALT, Modifiers::SHIFT, Modifiers::SUPER]
        .into_iter()
        .filter(|m| mods.contains(*m))
        .count();
    if mods.contains(Modifiers::SUPER) || count >= 2 {
        Ok(())
    } else {
        Err(TOO_COMMON.to_string())
    }
}

/// Every configured hotkey: the toggle first, then per-input hotkeys.
pub fn bindings(cfg: &Config) -> Vec<(String, HotkeyAction)> {
    let mut out = Vec::new();
    if let Some(accelerator) = &cfg.toggle_hotkey {
        out.push((accelerator.clone(), HotkeyAction::ToggleAll));
    }
    for (key, monitor) in &cfg.monitors {
        for (value, port) in &monitor.ports {
            if let (Some(accelerator), Ok(value)) = (&port.hotkey, value.parse::<u16>()) {
                out.push((accelerator.clone(), HotkeyAction::SwitchTo { key: MonitorKey(key.clone()), value }));
            }
        }
    }
    out
}

pub fn get(cfg: &Config, target: &HotkeyTarget) -> Option<String> {
    match target {
        HotkeyTarget::Toggle => cfg.toggle_hotkey.clone(),
        HotkeyTarget::Port { key, value } => cfg.monitor(key).and_then(|m| m.port(*value)).and_then(|p| p.hotkey.clone()),
    }
}

pub fn set(cfg: &mut Config, target: &HotkeyTarget, accelerator: Option<String>) {
    match target {
        HotkeyTarget::Toggle => cfg.toggle_hotkey = accelerator,
        HotkeyTarget::Port { key, value } => cfg.monitor_mut(key).port_mut(*value).hotkey = accelerator,
    }
}

/// True when `accelerator` is already bound to something other than `target`.
pub fn conflicts(cfg: &Config, target: &HotkeyTarget, accelerator: &str) -> bool {
    let Ok(id) = shortcut_id(accelerator) else {
        return false;
    };
    bindings(cfg).into_iter().any(|(other, action)| {
        let same_target = match (&action, target) {
            (HotkeyAction::ToggleAll, HotkeyTarget::Toggle) => true,
            (HotkeyAction::SwitchTo { key, value }, HotkeyTarget::Port { key: k, value: v }) => key == k && value == v,
            _ => false,
        };
        !same_target && shortcut_id(&other) == Ok(id)
    })
}

#[derive(Default)]
pub struct HotkeyRegistry {
    actions: Mutex<HashMap<u32, HotkeyAction>>,
}

impl HotkeyRegistry {
    pub fn action_for(&self, id: u32) -> Option<HotkeyAction> {
        self.actions.lock().unwrap().get(&id).cloned()
    }

    /// Replace every registered shortcut with `bindings`.
    /// Returns the accelerators the OS refused (usually taken by another program).
    pub fn apply<R: Runtime>(&self, app: &AppHandle<R>, bindings: Vec<(String, HotkeyAction)>) -> Vec<String> {
        let shortcuts = app.global_shortcut();
        let _ = shortcuts.unregister_all();
        let mut actions = self.actions.lock().unwrap();
        actions.clear();
        let mut failed = Vec::new();
        for (accelerator, action) in bindings {
            match Shortcut::from_str(&accelerator) {
                Ok(shortcut) => {
                    let id = shortcut.id();
                    if shortcuts.register(shortcut).is_ok() {
                        actions.insert(id, action);
                    } else {
                        failed.push(accelerator);
                    }
                }
                Err(_) => failed.push(accelerator),
            }
        }
        failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> MonitorKey {
        MonitorKey("DEL426A#UID1".into())
    }

    fn config() -> Config {
        let mut cfg = Config::default();
        cfg.monitor_mut(&key()).port_mut(0x1B).hotkey = Some("Ctrl+Alt+2".into());
        cfg
    }

    #[test]
    fn parses_accelerators() {
        assert!(shortcut_id("Ctrl+Alt+M").is_ok());
        assert_eq!(shortcut_id("ctrl+alt+m"), shortcut_id("Ctrl+Alt+M"));
        assert_eq!(shortcut_id("Ctrl+Alt+Nope"), Err(BAD_FORMAT.to_string()));
    }

    #[test]
    fn refuses_combinations_other_programs_rely_on() {
        assert_eq!(check_combination("Ctrl+C"), Err(TOO_COMMON.to_string()));
        assert_eq!(check_combination("Alt+F4"), Err(TOO_COMMON.to_string()));
        assert_eq!(check_combination("Shift+A"), Err(TOO_COMMON.to_string()));
        assert_eq!(check_combination("Ctrl+Alt+Nope"), Err(BAD_FORMAT.to_string()));
        assert!(check_combination("Ctrl+Alt+M").is_ok());
        assert!(check_combination("ctrl+shift+F5").is_ok());
        assert!(check_combination("Super+K").is_ok());
    }

    #[test]
    fn bindings_list_the_toggle_then_ports() {
        assert_eq!(
            bindings(&config()),
            vec![
                ("Ctrl+Alt+M".to_string(), HotkeyAction::ToggleAll),
                ("Ctrl+Alt+2".to_string(), HotkeyAction::SwitchTo { key: key(), value: 0x1B }),
            ]
        );
    }

    #[test]
    fn detects_conflicts_but_not_with_itself() {
        let cfg = config();
        let usb_c = HotkeyTarget::Port { key: key(), value: 0x1B };
        let dp = HotkeyTarget::Port { key: key(), value: 0x0F };
        assert!(conflicts(&cfg, &dp, "ctrl+alt+2"));
        assert!(conflicts(&cfg, &dp, "Ctrl+Alt+M"));
        assert!(!conflicts(&cfg, &usb_c, "Ctrl+Alt+2"));
        assert!(!conflicts(&cfg, &HotkeyTarget::Toggle, "Ctrl+Alt+M"));
        assert!(!conflicts(&cfg, &dp, "Ctrl+Alt+1"));
    }

    #[test]
    fn get_and_set_reach_the_right_field() {
        let mut cfg = config();
        let dp = HotkeyTarget::Port { key: key(), value: 0x0F };
        set(&mut cfg, &dp, Some("Ctrl+Alt+1".into()));
        assert_eq!(get(&cfg, &dp).as_deref(), Some("Ctrl+Alt+1"));
        set(&mut cfg, &HotkeyTarget::Toggle, None);
        assert_eq!(get(&cfg, &HotkeyTarget::Toggle), None);
    }
}
