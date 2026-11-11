//! The on-screen prompt shown when switching.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

use crate::view::{MonitorView, PortView};

pub const OSD_LABEL: &str = "osd";
pub const OSD_WIDTH: f64 = 260.0;
pub const OSD_HEIGHT: f64 = 210.0;
const VISIBLE_FOR: Duration = Duration::from_millis(1500);

/// Each show gets a number; only the latest one may hide the window.
static GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsdPayload {
    pub title: String,
    pub name: String,
    pub detail: String,
    pub icon: String,
}

pub fn for_port(title: &str, monitor: &MonitorView, port: &PortView) -> OsdPayload {
    OsdPayload {
        title: title.to_string(),
        name: port.display_name.clone(),
        detail: format!("{} · {}", port.port_label, monitor.label),
        icon: port.icon.to_string(),
    }
}

pub fn show(app: &AppHandle, payload: OsdPayload) {
    let Some(win) = app.get_webview_window(OSD_LABEL) else { return };
    // On the screen the mouse is on, centred, a little below the top.
    if let Ok(cursor) = app.cursor_position() {
        if let Ok(Some(monitor)) = app.monitor_from_point(cursor.x, cursor.y) {
            let wa = monitor.work_area();
            let width = (OSD_WIDTH * monitor.scale_factor()).round() as i32;
            let x = wa.position.x + (wa.size.width as i32 - width) / 2;
            let y = wa.position.y + (wa.size.height as f64 * 0.16).round() as i32;
            let _ = win.set_position(PhysicalPosition::new(x, y));
        }
    }
    let _ = app.emit_to(OSD_LABEL, "osd-show", payload);
    let _ = win.show();
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(VISIBLE_FOR);
        if GENERATION.load(Ordering::SeqCst) == generation {
            if let Some(win) = app.get_webview_window(OSD_LABEL) {
                let _ = win.hide();
            }
        }
    });
}
