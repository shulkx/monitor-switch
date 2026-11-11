use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::app::{actions, flyout, state::AppState, windows};
use crate::model::MonitorKey;
use crate::view::{MonitorView, PortView, View};

pub const TRAY_ID: &str = "main";

/// One line per monitor: "Dell U2723QE · 公司笔记本（USB-C）".
pub fn tooltip_text(view: &View) -> String {
    if view.monitors.is_empty() {
        return "MonitorSwitch · 没找到能控制的显示器".into();
    }
    view.monitors
        .iter()
        .map(|m| {
            let current = m
                .current
                .and_then(|c| m.ports.iter().find(|p| p.value == c))
                .map(|p| format!("{}（{}）", p.display_name, p.port_label))
                .unwrap_or_else(|| "读不到当前输入".into());
            let warning = if m.switch_error.is_some() { " · 上次切换没有成功" } else { "" };
            format!("{} · {}{}", m.label, current, warning)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn port_item(app: &AppHandle, monitor: &MonitorView, port: &PortView) -> tauri::Result<CheckMenuItem<Wry>> {
    CheckMenuItem::with_id(
        app,
        format!("switch|{}|{}", monitor.key.0, port.value),
        format!("{}（{}）", port.display_name, port.port_label),
        true,
        port.is_current,
        None::<&str>,
    )
}

/// One monitor → its inputs at the top level; several → one submenu each.
pub fn build_menu(app: &AppHandle, view: &View) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    if let [monitor] = view.monitors.as_slice() {
        for port in &monitor.ports {
            menu.append(&port_item(app, monitor, port)?)?;
        }
    } else {
        for monitor in &view.monitors {
            let submenu = Submenu::new(app, &monitor.label, true)?;
            for port in &monitor.ports {
                submenu.append(&port_item(app, monitor, port)?)?;
            }
            menu.append(&submenu)?;
        }
    }
    if !view.monitors.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?)?;
    Ok(menu)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let view = app.state::<AppState>().view();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tooltip_text(&view))
        .menu(&build_menu(app, &view)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                flyout::toggle_from_tray(tray.app_handle(), (position.x, position.y));
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn update(app: &AppHandle, view: &View) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let _ = tray.set_tooltip(Some(tooltip_text(view)));
    if let Ok(menu) = build_menu(app, view) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "settings" => {
            // Creating a window inside an event handler can deadlock on Windows; do it off-thread.
            let app = app.clone();
            std::thread::spawn(move || windows::open_settings(&app));
        }
        "quit" => app.exit(0),
        other => {
            let Some(rest) = other.strip_prefix("switch|") else { return };
            let Some((key, value)) = rest.rsplit_once('|') else { return };
            if let Ok(value) = value.parse::<u16>() {
                actions::perform_switch(app, MonitorKey(key.to_string()), value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::model::ConnectionKind;
    use crate::monitor_service::MonitorState;
    use crate::view::build_view;

    #[test]
    fn tooltip_lists_each_monitor() {
        let mut cfg = Config::default();
        let key = MonitorKey("DEL426A#UID1".into());
        cfg.monitor_mut(&key).port_mut(0x1B).name = Some("公司笔记本".into());
        let state = MonitorState {
            key,
            label: "Dell U2723QE".into(),
            connection: ConnectionKind::DisplayPort,
            inputs: vec![0x0F, 0x11, 0x1B],
            current: Some(0x1B),
            local_input: Some(0x0F),
            error: None,
            switch_error: None,
        };
        let view = build_view(&[state.clone()], &cfg, false, "PC");
        assert_eq!(tooltip_text(&view), "Dell U2723QE · 公司笔记本（USB-C）");

        let failed = MonitorState { switch_error: Some("切换可能没有生效".into()), ..state };
        let view = build_view(&[failed], &cfg, false, "PC");
        assert_eq!(tooltip_text(&view), "Dell U2723QE · 公司笔记本（USB-C） · 上次切换没有成功");
    }

    #[test]
    fn tooltip_without_monitors() {
        let view = build_view(&[], &Config::default(), false, "PC");
        assert_eq!(tooltip_text(&view), "MonitorSwitch · 没找到能控制的显示器");
    }
}
