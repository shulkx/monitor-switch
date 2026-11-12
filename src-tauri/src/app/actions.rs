//! What happens when the user switches, presses a hotkey, or the monitor changes.

use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::app::{osd, state::AppState, tray, windows};
use crate::hotkeys::{self, HotkeyAction, HotkeyTarget, IN_USE_ELSEWHERE, IN_USE_HERE};
use crate::model::MonitorKey;
use crate::monitor_service::ServiceEvent;
use crate::platform;
use crate::toggle::toggle_target;

/// How long the "正在切换到" prompt stays on this screen before the switch is sent (spec 5.2).
const PRE_SWITCH_PROMPT: Duration = Duration::from_millis(600);
/// Longest wait for the hotkey's keys to be let go before switching away anyway.
const KEY_RELEASE_WAIT: Duration = Duration::from_secs(2);

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
    if !leaving_this_computer {
        state.service.switch(key, value);
        return;
    }
    if view.show_osd {
        if let Some(port) = port {
            osd::show(app, osd::for_port("正在切换到", monitor, port));
        }
    }
    let prompt = view.show_osd;
    let app = app.clone();
    thread::spawn(move || {
        if prompt {
            thread::sleep(PRE_SWITCH_PROMPT);
        }
        wait_for_key_release();
        app.state::<AppState>().service.switch(key, value);
    });
}

/// Many monitors move their USB hub (and the keyboard on it) to the new input. Switching while
/// the hotkey's Ctrl is still down would leave Ctrl stuck on this computer.
fn wait_for_key_release() {
    let started = Instant::now();
    while platform::modifiers_down() && started.elapsed() < KEY_RELEASE_WAIT {
        thread::sleep(Duration::from_millis(20));
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

/// Push the current view to every window and refresh the tray, unless nothing changed:
/// rebuilding the tray menu while it is open drops the click, and re-rendering the
/// settings page closes any open drop-down.
pub fn publish_view(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let view = state.view();
    {
        let mut last = state.last_view.lock().unwrap();
        if last.as_ref() == Some(&view) {
            return;
        }
        *last = Some(view.clone());
    }
    tray::update(app, &view);
    let _ = app.emit("view-changed", view);
}

/// The settings page starts recording a hotkey. Our own hotkeys are turned off so pressing one
/// records it instead of switching the monitor. On Windows a keyboard hook also catches
/// combinations other programs own, which would otherwise never reach the page.
pub fn start_recording(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
    let handle = app.clone();
    platform::start_recording(move |accelerator| {
        // Called from inside the keyboard hook, which must return quickly.
        thread::spawn(move || {
            let _ = handle.emit_to(windows::SETTINGS_LABEL, "hotkey-recorded", accelerator);
        });
    });
}

/// Recording finished, was cancelled, or the settings window closed.
pub fn stop_recording(app: &AppHandle) {
    platform::stop_recording();
    apply_hotkeys(app);
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
        hotkeys::check_combination(a)?;
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
