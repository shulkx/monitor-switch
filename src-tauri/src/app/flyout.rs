//! The tray panel: show, hide, and keep it next to the tray icon.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

use crate::app::state::AppState;
use crate::platform;

pub const FLYOUT_LABEL: &str = "flyout";
pub const FLYOUT_WIDTH: f64 = 352.0;
pub const DEFAULT_HEIGHT: f64 = 360.0;
/// Space between the panel and the taskbar (or the overflow menu), in logical pixels.
const GAP: f64 = 8.0;
/// Closest the panel gets to the ends of the screen, in logical pixels.
const MARGIN: f64 = 12.0;

/// A rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Rect { left, top, right, bottom }
    }

    /// The tray event's rectangle, which Tauri reports in physical pixels.
    pub fn from_tauri(rect: tauri::Rect) -> Self {
        let p = rect.position.to_physical::<i32>(1.0);
        let s = rect.size.to_physical::<i32>(1.0);
        Rect::new(p.x, p.y, p.x + s.width, p.y + s.height)
    }

    fn center(&self) -> (i32, i32) {
        ((self.left + self.right) / 2, (self.top + self.bottom) / 2)
    }

    fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

/// The tray icon's rectangle; when Windows can't tell, the click point stands in.
pub fn icon_rect(rect: Rect, click: (f64, f64)) -> Rect {
    if rect.is_empty() {
        let (x, y) = (click.0.round() as i32, click.1.round() as i32);
        Rect::new(x, y, x, y)
    } else {
        rect
    }
}

/// The screen edge the taskbar is on: the side where the work area stops short of the monitor.
/// An auto-hiding taskbar takes no room, so then the edge nearest the icon wins.
pub fn taskbar_edge(monitor: Rect, work: Rect, icon: Rect) -> Edge {
    if work.bottom < monitor.bottom {
        return Edge::Bottom;
    }
    if work.top > monitor.top {
        return Edge::Top;
    }
    if work.left > monitor.left {
        return Edge::Left;
    }
    if work.right < monitor.right {
        return Edge::Right;
    }
    let (cx, cy) = icon.center();
    [
        (monitor.bottom - cy, Edge::Bottom),
        (cy - monitor.top, Edge::Top),
        (cx - monitor.left, Edge::Left),
        (monitor.right - cx, Edge::Right),
    ]
    .into_iter()
    .min_by_key(|(distance, _)| *distance)
    .map(|(_, edge)| edge)
    .unwrap_or(Edge::Bottom)
}

/// `value` within `min..=max`; `min` wins when the range is empty.
fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.min(max).max(min)
}

/// Top-left corner for a panel of `size` popping out of the tray `icon`. It goes past `host`
/// (the overflow menu holding the icon, if any) so neither covers the other, is centred on the
/// icon along the taskbar, and keeps `margin` away from the ends of the screen.
pub fn panel_origin(
    icon: Rect,
    host: Option<Rect>,
    monitor: Rect,
    work: Rect,
    size: (i32, i32),
    gap: i32,
    margin: i32,
) -> (i32, i32) {
    let avoid = host.unwrap_or(icon);
    let (w, h) = size;
    let (cx, cy) = icon.center();
    let along_x = clamp(cx - w / 2, work.left + margin, work.right - margin - w);
    let along_y = clamp(cy - h / 2, work.top + margin, work.bottom - margin - h);
    let (x, y) = match taskbar_edge(monitor, work, icon) {
        Edge::Bottom => (along_x, avoid.top.min(work.bottom) - gap - h),
        Edge::Top => (along_x, avoid.bottom.max(work.top) + gap),
        Edge::Left => (avoid.right.max(work.left) + gap, along_y),
        Edge::Right => (avoid.left.min(work.right) - gap - w, along_y),
    };
    (clamp(x, work.left, work.right - w), clamp(y, work.top, work.bottom - h))
}

/// What the panel is placed against.
#[derive(Clone, Copy, Debug)]
struct Anchor {
    icon: Rect,
    /// The overflow menu holding the icon, looked up at click time: by the time the panel
    /// resizes to its content the menu may have closed.
    host: Option<Rect>,
}

#[derive(Default)]
pub struct FlyoutState {
    anchor: Mutex<Option<Anchor>>,
    last_hidden: Mutex<Option<Instant>>,
    /// Content height reported by the page, in logical pixels.
    height: Mutex<Option<f64>>,
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(FLYOUT_LABEL)
}

/// Left click on the tray icon.
pub fn toggle_from_tray(app: &AppHandle, icon: Rect) {
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
    let (cx, cy) = icon.center();
    let host = platform::tray_icon_host(cx, cy).map(|[l, t, r, b]| Rect::new(l, t, r, b));
    *state.flyout.anchor.lock().unwrap() = Some(Anchor { icon, host });
    show(app);
}

/// Show the panel in the bottom-right corner of the primary screen (used by single-instance).
pub fn show_default(app: &AppHandle) {
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let wa = monitor.work_area();
        let x = wa.position.x + wa.size.width as i32 - 1;
        let y = wa.position.y + wa.size.height as i32 - 1;
        *app.state::<AppState>().flyout.anchor.lock().unwrap() =
            Some(Anchor { icon: Rect::new(x, y, x, y), host: None });
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
    let height = state.flyout.height.lock().unwrap().unwrap_or(DEFAULT_HEIGHT);
    let _ = win.set_size(LogicalSize::new(FLYOUT_WIDTH, height));
    let Some(anchor) = *state.flyout.anchor.lock().unwrap() else { return };
    let (cx, cy) = anchor.icon.center();
    let monitor = app
        .monitor_from_point(cx as f64, cy as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };
    let (pos, size, wa) = (monitor.position(), monitor.size(), monitor.work_area());
    let bounds = Rect::new(pos.x, pos.y, pos.x + size.width as i32, pos.y + size.height as i32);
    let work = Rect::new(
        wa.position.x,
        wa.position.y,
        wa.position.x + wa.size.width as i32,
        wa.position.y + wa.size.height as i32,
    );
    let px = |logical: f64| (logical * monitor.scale_factor()).round() as i32;
    let (x, y) = panel_origin(anchor.icon, anchor.host, bounds, work, (px(FLYOUT_WIDTH), px(height)), px(GAP), px(MARGIN));
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON_4K: Rect = Rect::new(0, 0, 3840, 2160);
    const WORK_4K: Rect = Rect::new(0, 0, 3840, 2100);
    const PANEL: (i32, i32) = (528, 480);

    #[test]
    fn icon_on_a_bottom_taskbar_puts_the_panel_right_above_it() {
        let icon = Rect::new(3480, 2100, 3516, 2160);
        assert_eq!(panel_origin(icon, None, MON_4K, WORK_4K, PANEL, 12, 18), (3498 - 264, 2100 - 12 - 480));
    }

    #[test]
    fn icon_in_the_overflow_menu_puts_the_panel_above_the_menu() {
        // Measured on Windows 10 at 150 %: the menu and the icon's cell in its first row.
        let menu = Rect::new(3300, 1956, 3492, 2100);
        let icon = Rect::new(3396, 1956, 3444, 2004);
        assert_eq!(panel_origin(icon, Some(menu), MON_4K, WORK_4K, PANEL, 12, 18), (3420 - 264, 1956 - 12 - 480));
    }

    #[test]
    fn stays_off_the_right_end_of_the_screen() {
        let icon = Rect::new(3800, 2100, 3836, 2160);
        assert_eq!(panel_origin(icon, None, MON_4K, WORK_4K, PANEL, 12, 18).0, 3840 - 18 - 528);
    }

    const MON: Rect = Rect::new(0, 0, 1920, 1080);

    #[test]
    fn top_taskbar() {
        let work = Rect::new(0, 40, 1920, 1080);
        let icon = Rect::new(1800, 0, 1824, 40);
        assert_eq!(panel_origin(icon, None, MON, work, (352, 400), 8, 12), (1920 - 12 - 352, 48));
    }

    #[test]
    fn left_taskbar() {
        let work = Rect::new(60, 0, 1920, 1080);
        let icon = Rect::new(0, 1000, 60, 1040);
        assert_eq!(panel_origin(icon, None, MON, work, (352, 400), 8, 12), (68, 1080 - 12 - 400));
    }

    #[test]
    fn right_taskbar() {
        let work = Rect::new(0, 0, 1860, 1080);
        let icon = Rect::new(1860, 1000, 1920, 1040);
        assert_eq!(panel_origin(icon, None, MON, work, (352, 400), 8, 12), (1860 - 8 - 352, 1080 - 12 - 400));
    }

    #[test]
    fn auto_hiding_taskbar_uses_the_nearest_edge() {
        let icon = Rect::new(1700, 1040, 1724, 1080);
        assert_eq!(taskbar_edge(MON, MON, icon), Edge::Bottom);
        assert_eq!(panel_origin(icon, None, MON, MON, (352, 400), 8, 12), (1712 - 176, 1040 - 8 - 400));
    }

    #[test]
    fn secondary_monitor() {
        let mon = Rect::new(1920, 0, 3840, 1080);
        let work = Rect::new(1920, 0, 3840, 1040);
        let icon = Rect::new(3700, 1040, 3724, 1080);
        assert_eq!(panel_origin(icon, None, mon, work, (352, 400), 8, 12), (3840 - 12 - 352, 1040 - 8 - 400));
    }

    #[test]
    fn a_panel_taller_than_the_room_above_the_menu_stays_on_screen() {
        let mon = Rect::new(0, 0, 1366, 768);
        let work = Rect::new(0, 0, 1366, 728);
        let menu = Rect::new(1100, 600, 1250, 728);
        let icon = Rect::new(1150, 610, 1180, 640);
        assert_eq!(panel_origin(icon, Some(menu), mon, work, (352, 700), 8, 12), (1165 - 176, 0));
    }

    #[test]
    fn a_missing_icon_rect_falls_back_to_the_click() {
        assert_eq!(icon_rect(Rect::new(0, 0, 0, 0), (3400.0, 2130.0)), Rect::new(3400, 2130, 3400, 2130));
        assert_eq!(icon_rect(Rect::new(1, 2, 3, 4), (3400.0, 2130.0)), Rect::new(1, 2, 3, 4));
    }
}
