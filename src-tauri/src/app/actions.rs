//! What happens when the user switches, presses a hotkey, or the monitor changes.

use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::app::{osd, state::AppState, tray};
use crate::hotkeys::{self, HotkeyAction, HotkeyTarget, IN_USE_ELSEWHERE, IN_USE_HERE};
use crate::model::MonitorKey;
use crate::monitor_service::ServiceEvent;
use crate::toggle::toggle_target;

/// How long the "正在切换到" prompt stays on this screen before the switch is sent (spec 5.2).
const PRE_SWITCH_PROMPT: Duration = Duration::from_millis(600);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchFailedPayload {
    pub key: String,
    pub value: u16,
    pub reason: String,
}

pub fn perform_switch(app: &AppHandle, key: MonitorKey, value: u16) {
    let state = app.state::<AppState>();
    let view = state.view();
    let Some(monitor) = view.monitors.iter().find(|m| m.key == key) else { return };
    let port = monitor.ports.iter().find(|p| p.value == value);
    let leaving_this_computer = !port.map(|p| p.is_local).unwrap_or(false);
    if view.show_osd && leaving_this_computer {
        if let Some(port) = port {
            osd::show(app, osd::for_port("正在切换到", monitor, port));
        }
        let app = app.clone();
        thread::spawn(move || {
            thread::sleep(PRE_SWITCH_PROMPT);
            app.state::<AppState>().service.switch(key, value);
        });
    } else {
        state.service.switch(key, value);
    }
}

/// The toggle hotkey: every monitor with a toggle pair switches to the other input of its pair.
pub fn toggle_all(app: &AppHandle) {
    let view = app.state::<AppState>().view();
    for monitor in &view.monitors {
        if let Some(pair) = monitor.toggle_pair {
            perform_switch(app, monitor.key.clone(), toggle_target(monitor.current, pair));
        }
    }
}

pub fn on_hotkey(app: &AppHandle, shortcut_id: u32) {
    let Some(state) = app.try_state::<AppState>() else { return };
    match state.hotkeys.action_for(shortcut_id) {
        Some(HotkeyAction::ToggleAll) => toggle_all(app),
        Some(HotkeyAction::SwitchTo { key, value }) => perform_switch(app, key, value),
        None => {}
    }
}

/// Runs on the MonitorService worker thread.
pub fn handle_service_event(app: &AppHandle, event: ServiceEvent) {
    let Some(state) = app.try_state::<AppState>() else { return };
    match event {
        ServiceEvent::Snapshot(_) => publish_view(app),
        ServiceEvent::InputChanged { key, to, local_input, .. } => {
            let view = state.view();
            // The monitor now shows this computer: confirm it on this computer's screen.
            if view.show_osd && local_input == Some(to) {
                if let Some(monitor) = view.monitors.iter().find(|m| m.key == key) {
                    if let Some(port) = monitor.ports.iter().find(|p| p.value == to) {
                        osd::show(app, osd::for_port("已切换到", monitor, port));
                    }
                }
            }
            publish_view(app);
        }
        ServiceEvent::SwitchFailed { key, value, reason } => {
            let _ = app.emit("switch-failed", SwitchFailedPayload { key: key.0, value, reason });
        }
    }
}

/// Push the current view to every window and refresh the tray.
pub fn publish_view(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let view = state.view();
    tray::update(app, &view);
    let _ = app.emit("view-changed", view);
}

/// Register every configured hotkey again and remember which ones the OS refused.
pub fn apply_hotkeys(app: &AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let bindings = hotkeys::bindings(&state.config.lock().unwrap());
    let failed = state.hotkeys.apply(app, bindings);
    *state.failed_hotkeys.lock().unwrap() = failed.clone();
    failed
}

pub fn set_hotkey(app: &AppHandle, target: HotkeyTarget, accelerator: Option<String>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let accelerator = accelerator.map(|a| a.trim().to_string()).filter(|a| !a.is_empty());
    if let Some(a) = &accelerator {
        hotkeys::shortcut_id(a)?;
    }
    let previous = {
        let mut cfg = state.config.lock().unwrap();
        if let Some(a) = &accelerator {
            if hotkeys::conflicts(&cfg, &target, a) {
                return Err(IN_USE_HERE.into());
            }
        }
        let previous = hotkeys::get(&cfg, &target);
        hotkeys::set(&mut cfg, &target, accelerator.clone());
        previous
    };
    let failed = apply_hotkeys(app);
    if let Some(a) = &accelerator {
        if failed.contains(a) {
            hotkeys::set(&mut state.config.lock().unwrap(), &target, previous);
            apply_hotkeys(app);
            return Err(IN_USE_ELSEWHERE.into());
        }
    }
    state.update(|_| {})?;
    publish_view(app);
    Ok(())
}

/// Make the OS autostart entry match the setting. Skipped in debug builds, which would
/// register the development executable.
pub fn sync_autostart(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let want = app.state::<AppState>().config.lock().unwrap().autostart;
    let launcher = app.autolaunch();
    let _ = if want { launcher.enable() } else { launcher.disable() };
}
