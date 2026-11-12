//! Commands the pages call with `invoke`. Argument names arrive camelCase from JavaScript.

use tauri::{AppHandle, Manager, State};

use crate::app::{actions, flyout, state::AppState, windows};
use crate::hotkeys::HotkeyTarget;
use crate::model::MonitorKey;
use crate::view::View;

#[tauri::command]
pub fn get_view(state: State<'_, AppState>) -> View {
    state.view()
}

#[tauri::command]
pub fn refresh(state: State<'_, AppState>) {
    state.service.refresh();
}

#[tauri::command]
pub fn switch_input(app: AppHandle, key: String, value: u16) {
    actions::perform_switch(&app, MonitorKey(key), value);
}

#[tauri::command]
pub fn toggle(app: AppHandle) {
    actions::toggle_all(&app);
}

#[tauri::command]
pub fn set_port_name(app: AppHandle, key: String, value: u16, name: Option<String>) -> Result<(), String> {
    let name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    app.state::<AppState>()
        .update(|cfg| cfg.monitor_mut(&MonitorKey(key)).port_mut(value).name = name)?;
    actions::publish_view(&app);
    Ok(())
}

#[tauri::command]
pub fn set_port_hotkey(app: AppHandle, key: String, value: u16, accelerator: Option<String>) -> Result<(), String> {
    actions::set_hotkey(&app, HotkeyTarget::Port { key: MonitorKey(key), value }, accelerator)
}

#[tauri::command]
pub fn set_toggle_hotkey(app: AppHandle, accelerator: Option<String>) -> Result<(), String> {
    actions::set_hotkey(&app, HotkeyTarget::Toggle, accelerator)
}

/// The settings page is about to record a hotkey.
#[tauri::command]
pub fn start_hotkey_recording(app: AppHandle) {
    actions::start_recording(&app);
}

/// Recording finished or was cancelled.
#[tauri::command]
pub fn stop_hotkey_recording(app: AppHandle) {
    actions::stop_recording(&app);
}

#[tauri::command]
pub fn set_toggle_pair(app: AppHandle, key: String, pair: Option<[u16; 2]>) -> Result<(), String> {
    if let Some([a, b]) = pair {
        if a == b {
            return Err("两个接口要不一样".into());
        }
    }
    app.state::<AppState>()
        .update(|cfg| cfg.monitor_mut(&MonitorKey(key)).toggle_pair = pair)?;
    actions::publish_view(&app);
    Ok(())
}

#[tauri::command]
pub fn set_local_override(app: AppHandle, key: String, value: Option<u16>) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.update(|cfg| cfg.monitor_mut(&MonitorKey(key)).local_port_override = value)?;
    state.service.refresh();
    Ok(())
}

#[tauri::command]
pub fn set_read_capabilities(app: AppHandle, key: String, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.update(|cfg| {
        let m = cfg.monitor_mut(&MonitorKey(key));
        m.read_capabilities = enabled;
        m.caps_read_pending = false;
        if enabled {
            m.cached_capabilities = None;
        }
    })?;
    state.service.refresh();
    Ok(())
}

#[tauri::command]
pub fn set_option(app: AppHandle, name: String, value: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    match name.as_str() {
        "showOsd" => state.update(|cfg| cfg.show_osd = value)?,
        "lanDiscovery" => state.update(|cfg| cfg.lan_discovery = value)?,
        "autostart" => {
            state.update(|cfg| cfg.autostart = value)?;
            actions::sync_autostart(&app);
        }
        other => return Err(format!("unknown option {other}")),
    }
    actions::publish_view(&app);
    Ok(())
}

/// Async: creating a window from a synchronous command deadlocks on Windows.
#[tauri::command]
pub async fn open_settings(app: AppHandle) {
    windows::open_settings(&app);
}

#[tauri::command]
pub fn hide_flyout(app: AppHandle) {
    flyout::hide(&app);
}

#[tauri::command]
pub fn fit_flyout(app: AppHandle, height: f64) {
    flyout::fit(&app, height);
}
