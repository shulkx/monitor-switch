use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::app::{flyout, osd};

pub const SETTINGS_LABEL: &str = "settings";

pub fn create_flyout(app: &AppHandle) -> tauri::Result<()> {
    let win = WebviewWindowBuilder::new(app, flyout::FLYOUT_LABEL, WebviewUrl::App("flyout.html".into()))
        .title("MonitorSwitch")
        .inner_size(flyout::FLYOUT_WIDTH, flyout::DEFAULT_HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .build()?;
    let handle = app.clone();
    win.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => flyout::hide(&handle),
        // Alt+F4 would destroy the panel for good; hide it instead.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            flyout::hide(&handle);
        }
        _ => {}
    });
    Ok(())
}

pub fn create_osd(app: &AppHandle) -> tauri::Result<()> {
    let win = WebviewWindowBuilder::new(app, osd::OSD_LABEL, WebviewUrl::App("osd.html".into()))
        .title("MonitorSwitch")
        .inner_size(osd::OSD_WIDTH, osd::OSD_HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .focusable(false)
        .focused(false)
        .visible(false)
        .build()?;
    win.set_ignore_cursor_events(true)?;
    let handle = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Some(win) = handle.get_webview_window(osd::OSD_LABEL) {
                let _ = win.hide();
            }
        }
    });
    Ok(())
}

/// Must not be called from a synchronous command on Windows (window creation would deadlock);
/// the `open_settings` command is async for that reason.
pub fn open_settings(app: &AppHandle) {
    flyout::hide(app);
    if let Some(win) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("settings.html".into()))
        .title("MonitorSwitch 设置")
        .inner_size(520.0, 680.0)
        .build();
}
