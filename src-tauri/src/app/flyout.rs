//! The tray panel: show, hide, and keep it next to the tray icon.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::app::state::AppState;

pub const FLYOUT_LABEL: &str = "flyout";
pub const FLYOUT_WIDTH: f64 = 352.0;
pub const DEFAULT_HEIGHT: f64 = 360.0;
const MARGIN: f64 = 12.0;

#[derive(Default)]
pub struct FlyoutState {
    /// Where the tray icon was clicked, in physical pixels.
    anchor: Mutex<Option<(f64, f64)>>,
    last_hidden: Mutex<Option<Instant>>,
    /// Content height reported by the page, in logical pixels.
    height: Mutex<Option<f64>>,
}

/// Top-left corner for a panel of `size` (physical px) shown for a click at `click`,
/// inside the monitor work area `work` = (x, y, width, height).
/// Taskbar at the bottom → panel above the click; menu bar at the top → panel below it.
pub fn origin(click: (f64, f64), work: (i32, i32, u32, u32), size: (u32, u32), margin: i32) -> (i32, i32) {
    let (wx, wy, ww, wh) = (work.0, work.1, work.2 as i32, work.3 as i32);
    let (w, h) = (size.0 as i32, size.1 as i32);
    let min_x = wx + margin;
    let max_x = (wx + ww - w - margin).max(min_x);
    let x = (click.0.round() as i32 - w / 2).clamp(min_x, max_x);
    let min_y = wy + margin;
    let max_y = (wy + wh - h - margin).max(min_y);
    let taskbar_at_bottom = click.1.round() as i32 >= wy + wh / 2;
    let y = if taskbar_at_bottom { max_y } else { min_y };
    (x, y)
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(FLYOUT_LABEL)
}

/// Left click on the tray icon.
pub fn toggle_from_tray(app: &AppHandle, click: (f64, f64)) {
    let Some(win) = window(app) else { return };
    if win.is_visible().unwrap_or(false) {
        hide(app);
        return;
    }
    // Clicking the tray icon while the panel is open first blurs it (which hides it),
    // then delivers the click. Don't reopen it in that case.
    let state = app.state::<AppState>();
    let just_hidden = state
        .flyout
        .last_hidden
        .lock()
        .unwrap()
        .map(|t| t.elapsed() < Duration::from_millis(300))
        .unwrap_or(false);
    if just_hidden {
        return;
    }
    *state.flyout.anchor.lock().unwrap() = Some(click);
    show(app);
}

/// Show the panel near the bottom-right corner of the primary screen (used by single-instance).
pub fn show_default(app: &AppHandle) {
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let wa = monitor.work_area();
        let corner = (
            (wa.position.x + wa.size.width as i32 - 1) as f64,
            (wa.position.y + wa.size.height as i32 - 1) as f64,
        );
        *app.state::<AppState>().flyout.anchor.lock().unwrap() = Some(corner);
    }
    show(app);
}

pub fn show(app: &AppHandle) {
    let Some(win) = window(app) else { return };
    place(app, &win);
    let _ = win.show();
    let _ = win.set_focus();
    let state = app.state::<AppState>();
    state.service.refresh();
    let _ = app.emit_to(FLYOUT_LABEL, "view-changed", state.view());
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = window(app) {
        let _ = win.hide();
    }
    *app.state::<AppState>().flyout.last_hidden.lock().unwrap() = Some(Instant::now());
}

/// The page reports its content height after every render.
pub fn fit(app: &AppHandle, height: f64) {
    *app.state::<AppState>().flyout.height.lock().unwrap() = Some(height.clamp(120.0, 900.0));
    if let Some(win) = window(app) {
        place(app, &win);
    }
}

fn place(app: &AppHandle, win: &WebviewWindow) {
    let state = app.state::<AppState>();
    let anchor = state.flyout.anchor.lock().unwrap().unwrap_or((0.0, 0.0));
    let height = state.flyout.height.lock().unwrap().unwrap_or(DEFAULT_HEIGHT);
    let monitor = app
        .monitor_from_point(anchor.0, anchor.1)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let scale = monitor.scale_factor();
    let _ = win.set_size(LogicalSize::new(FLYOUT_WIDTH, height));
    let wa = monitor.work_area();
    let size = ((FLYOUT_WIDTH * scale).round() as u32, (height * scale).round() as u32);
    let (x, y) = origin(
        anchor,
        (wa.position.x, wa.position.y, wa.size.width, wa.size.height),
        size,
        (MARGIN * scale).round() as i32,
    );
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_taskbar_puts_the_panel_above_the_click() {
        assert_eq!(origin((1880.0, 1060.0), (0, 0, 1920, 1032), (352, 400), 12), (1920 - 352 - 12, 1032 - 400 - 12));
    }

    #[test]
    fn top_menu_bar_puts_the_panel_below_the_click() {
        assert_eq!(origin((800.0, 10.0), (0, 25, 1440, 875), (352, 400), 12), (800 - 176, 25 + 12));
    }

    #[test]
    fn stays_inside_the_left_edge() {
        assert_eq!(origin((50.0, 1060.0), (0, 0, 1920, 1032), (352, 400), 12).0, 12);
    }

    #[test]
    fn works_on_a_secondary_monitor() {
        assert_eq!(origin((3800.0, 1060.0), (1920, 0, 1920, 1032), (352, 400), 12), (1920 + 1920 - 352 - 12, 620));
    }
}
