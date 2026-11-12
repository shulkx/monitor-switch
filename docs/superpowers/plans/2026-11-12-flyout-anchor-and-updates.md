# 面板贴着托盘图标弹出 + 更新提醒 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal：** 面板按托盘图标（以及它所在的溢出菜单）定位，不再盖住图标；新增"只提醒"的更新功能和带签名的发布脚本，发布 0.1.2。

**Architecture：**
- **定位**：用纯函数 `panel_origin` 算坐标，输入是图标矩形、宿主矩形、显示器范围、工作区和面板尺寸。宿主矩形在点击时由 `platform::tray_icon_host` 用 Win32 查出。
- **更新**：
  - `updates.rs` 放纯状态逻辑；
  - `app/updater.rs` 负责 `tauri-plugin-updater` 的检查、下载、安装；
  - 状态放进 `View`，托盘菜单、面板、设置页都从 `View` 渲染。
- **发布**：`scripts/release.mjs` 带签名构建，并用 `scripts/minisign.mjs` 先验签，再生成 `latest.json`。

**Tech Stack：** Rust（Tauri 2.12、tauri-plugin-updater 2）、TypeScript + Vite、Node 24（`node:test`、`node:crypto`）、Win32 FFI。

设计文档：`docs/superpowers/specs/2026-11-12-flyout-anchor-and-updates-design.md`

---

## 文件结构

| 文件 | 改动 |
| --- | --- |
| `src-tauri/src/app/flyout.rs` | `Rect`、`Edge`、`taskbar_edge`、`panel_origin`；`Anchor` 状态；按图标定位 |
| `src-tauri/src/app/tray.rs` | 点击时传图标矩形；更新菜单项 |
| `src-tauri/src/platform/win/ffi.rs` | `WindowFromPoint`、`GetAncestor`、`GetClassNameW`、`GetWindowRect`、`ShellExecuteW` |
| `src-tauri/src/platform/win/mod.rs`、`platform/mod.rs` | `tray_icon_host`、`open_url` |
| `src-tauri/src/updates.rs`（新） | `UpdateStatus`、`after_check`、`percent`、`worth_showing` |
| `src-tauri/src/app/updater.rs`（新） | `UpdateState`、后台检查、`check`、`install` |
| `src-tauri/src/config.rs` | `check_updates` |
| `src-tauri/src/view.rs`、`app/state.rs` | `appVersion`、`update`、`checkUpdates` |
| `src-tauri/src/commands.rs`、`lib.rs`、`app/mod.rs` | 新命令、注册插件、启动后台检查 |
| `src-tauri/Cargo.toml`、`tauri.conf.json`、`src-tauri/release.conf.json`（新） | 插件依赖、公钥和地址、发布时打开 `createUpdaterArtifacts` |
| `src/shared/api.ts`、`src/flyout/*`、`src/settings/*` | 更新条、"关于"区 |
| `scripts/minisign.mjs`、`scripts/minisign.test.mjs`、`scripts/release.mjs`（新） | 验签、发布 |
| `package.json`、`README.md` | `release`、`test:scripts` 脚本；文档 |

---

### Task 1: 面板定位的纯函数

**Files:** Modify `src-tauri/src/app/flyout.rs`

- [ ] **Step 1: 写测试**（替换旧的 `origin` 测试）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MON_4K: Rect = Rect { left: 0, top: 0, right: 3840, bottom: 2160 };
    const WORK_4K: Rect = Rect { left: 0, top: 0, right: 3840, bottom: 2100 };
    const PANEL: (i32, i32) = (528, 480);

    #[test]
    fn icon_on_a_bottom_taskbar_puts_the_panel_right_above_it() {
        let icon = Rect::new(3480, 2100, 3516, 2160);
        assert_eq!(panel_origin(icon, None, MON_4K, WORK_4K, PANEL, 12, 18), (3498 - 264, 2100 - 12 - 480));
    }

    #[test]
    fn icon_in_the_overflow_menu_puts_the_panel_above_the_menu() {
        let menu = Rect::new(3300, 1956, 3492, 2100);
        let icon = Rect::new(3396, 1956, 3444, 2004);
        assert_eq!(panel_origin(icon, Some(menu), MON_4K, WORK_4K, PANEL, 12, 18), (3420 - 264, 1956 - 12 - 480));
    }

    #[test]
    fn stays_off_the_right_end_of_the_screen() {
        let icon = Rect::new(3800, 2100, 3836, 2160);
        assert_eq!(panel_origin(icon, None, MON_4K, WORK_4K, PANEL, 12, 18).0, 3840 - 18 - 528);
    }

    const MON: Rect = Rect { left: 0, top: 0, right: 1920, bottom: 1080 };

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
```

- [ ] **Step 2:** `cargo test --manifest-path src-tauri/Cargo.toml flyout`。预期编译失败，提示 `panel_origin` 未定义。
- [ ] **Step 3: 实现**

```rust
/// A rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self { Rect { left, top, right, bottom } }
    fn center(&self) -> (i32, i32) { ((self.left + self.right) / 2, (self.top + self.bottom) / 2) }
    fn is_empty(&self) -> bool { self.right <= self.left || self.bottom <= self.top }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge { Bottom, Top, Left, Right }

/// The tray icon's rectangle; Windows sometimes can't tell, then the click point stands in.
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
    if work.bottom < monitor.bottom { return Edge::Bottom; }
    if work.top > monitor.top { return Edge::Top; }
    if work.left > monitor.left { return Edge::Left; }
    if work.right < monitor.right { return Edge::Right; }
    let (cx, cy) = icon.center();
    [(monitor.bottom - cy, Edge::Bottom), (cy - monitor.top, Edge::Top), (cx - monitor.left, Edge::Left), (monitor.right - cx, Edge::Right)]
        .into_iter().min_by_key(|(distance, _)| *distance).map(|(_, edge)| edge).unwrap_or(Edge::Bottom)
}

/// `value` within `min..=max`; `min` wins when the range is empty.
fn clamp(value: i32, min: i32, max: i32) -> i32 { value.min(max).max(min) }

/// Top-left corner for a panel of `size` popping out of the tray `icon`. It goes past `host`
/// (the overflow menu holding the icon, if any) so neither covers the other, centred on the icon
/// along the taskbar and kept `margin` away from the screen ends.
pub fn panel_origin(icon: Rect, host: Option<Rect>, monitor: Rect, work: Rect, size: (i32, i32), gap: i32, margin: i32) -> (i32, i32) {
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
```

- [ ] **Step 4:** 再跑同一条测试，预期全部 PASS。

### Task 2: 查找图标所在的窗口，接入托盘点击

**Files:** Modify `platform/win/ffi.rs`、`platform/win/mod.rs`、`platform/mod.rs`、`app/flyout.rs`、`app/tray.rs`

- [ ] **Step 1：** 在 `ffi.rs` 的 user32 块里加声明：
  - `WindowFromPoint(point: POINT) -> isize`
  - `GetAncestor(hwnd: isize, flags: u32) -> isize`
  - `GetClassNameW(hwnd: isize, name: *mut u16, max: i32) -> i32`
  - `GetWindowRect(hwnd: isize, rect: *mut RECT) -> i32`
  - 常量 `GA_ROOT: u32 = 2`

  另起一个 shell32 块：`ShellExecuteW(hwnd: isize, op: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> isize`，加常量 `SW_SHOWNORMAL = 1`。
- [ ] **Step 2：** 在 `platform/win/mod.rs` 里实现：

```rust
/// The pop-up holding the tray icon at (x, y), i.e. the overflow menu, as [left, top, right, bottom];
/// `None` when the icon sits on a taskbar. Call it right after the click, while the menu is open.
pub fn tray_icon_host(x: i32, y: i32) -> Option<[i32; 4]> {
    unsafe {
        let root = ffi::GetAncestor(ffi::WindowFromPoint(ffi::POINT { x, y }), ffi::GA_ROOT);
        if root == 0 { return None; }
        let mut class = [0u16; 64];
        let len = ffi::GetClassNameW(root, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..len]);
        if class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd" { return None; }
        let mut pid = 0;
        ffi::GetWindowThreadProcessId(root, &mut pid);
        if pid == ffi::GetCurrentProcessId() { return None; }
        let mut r = ffi::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if ffi::GetWindowRect(root, &mut r) == 0 { return None; }
        Some([r.left, r.top, r.right, r.bottom])
    }
}

/// Open a web page in the default browser.
pub fn open_url(url: &str) {
    let (op, file) = (ffi::to_wide("open"), ffi::to_wide(url));
    unsafe { ffi::ShellExecuteW(0, op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), ffi::SW_SHOWNORMAL) };
}
```

  然后在 `platform/mod.rs` 里转出这两个函数；非 Windows 平台写成 `tray_icon_host → None`，`open_url` 什么都不做。
- [ ] **Step 3：** 改 `flyout.rs`：
  - `FlyoutState.anchor: Mutex<Option<Anchor>>`，`Anchor { icon: Rect, host: Option<Rect> }`。
  - `toggle_from_tray(app, icon: Rect)` 在 `show` 之前调用 `platform::tray_icon_host(icon 中心)`，把结果存进 anchor。
  - `show_default` 用主屏工作区右下角的一个零尺寸矩形作 anchor。
  - `place()` 用 `monitor.position()`、`monitor.size()` 和 `work_area()` 构造两个 `Rect`，再调用 `panel_origin`，常量 `GAP = 8.0`、`MARGIN = 12.0`，乘以缩放比换成物理像素。
- [ ] **Step 4：** 在 `tray.rs` 里，Click 事件同时取出 `rect` 和 `position`，算出 `icon_rect(Rect::from_tauri(rect), (position.x, position.y))` 再传给 `toggle_from_tray`。`Rect::from_tauri` 的写法：`rect.position.to_physical::<i32>(1.0)` 和 `rect.size.to_physical::<i32>(1.0)`。
- [ ] **Step 5：** `cargo test` 全部通过，然后提交：`fix: open the panel above the tray icon and its overflow menu`。

### Task 3: 更新状态（纯逻辑）、设置项、View 字段

**Files:** Create `src-tauri/src/updates.rs`；Modify `lib.rs`（`pub mod updates;`）、`config.rs`、`view.rs`、`app/state.rs`

- [ ] **Step 1：** 测试（写在 `updates.rs` 里）：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    fn available(v: &str) -> UpdateStatus { UpdateStatus::Available { version: v.into(), notes: None } }

    #[test]
    fn a_found_version_becomes_available() {
        assert_eq!(after_check(&UpdateStatus::Idle, Ok(Some(("0.1.3".into(), None))), false), available("0.1.3"));
        assert_eq!(after_check(&available("0.1.3"), Ok(None), false), UpdateStatus::UpToDate);
    }

    #[test]
    fn background_failures_keep_what_was_shown() {
        assert_eq!(after_check(&available("0.1.3"), Err("offline".into()), false), available("0.1.3"));
        assert_eq!(after_check(&UpdateStatus::Idle, Err("offline".into()), false), UpdateStatus::Idle);
    }

    #[test]
    fn manual_failures_say_why() {
        assert_eq!(
            after_check(&UpdateStatus::Checking, Err("offline".into()), true),
            UpdateStatus::Failed { message: "检查更新失败：offline".into(), version: None }
        );
    }

    #[test]
    fn a_download_in_progress_is_not_interrupted() {
        let downloading = UpdateStatus::Downloading { version: "0.1.3".into(), percent: Some(40) };
        assert_eq!(after_check(&downloading, Ok(Some(("0.1.4".into(), None))), false), downloading);
    }

    #[test]
    fn progress() {
        assert_eq!(percent(50, Some(200)), Some(25));
        assert_eq!(percent(300, Some(200)), Some(100));
        assert_eq!(percent(50, None), None);
        assert!(worth_showing(None, Some(0)));
        assert!(!worth_showing(Some(10), Some(14)));
        assert!(worth_showing(Some(10), Some(15)));
        assert!(worth_showing(Some(97), Some(100)));
        assert!(!worth_showing(None, None));
    }

    #[test]
    fn serializes_for_the_pages() {
        let json = serde_json::to_value(UpdateStatus::Downloading { version: "0.1.3".into(), percent: Some(5) }).unwrap();
        assert_eq!(json, serde_json::json!({ "state": "downloading", "version": "0.1.3", "percent": 5 }));
        assert_eq!(serde_json::to_value(UpdateStatus::UpToDate).unwrap(), serde_json::json!({ "state": "upToDate" }));
    }
}
```

- [ ] **Step 2：** 实现：

```rust
//! What the update reminder shows (design 2026-11-12, section 3).
use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available { version: String, notes: Option<String> },
    Downloading { version: String, percent: Option<u8> },
    Failed { message: String, version: Option<String> },
}

/// A newer version and its release notes, or `None` when this one is the latest.
pub type Found = Option<(String, Option<String>)>;

/// The state after a check. A failed background check keeps what was shown; a check the user
/// asked for says why it failed. A download in progress is never replaced.
pub fn after_check(previous: &UpdateStatus, result: Result<Found, String>, manual: bool) -> UpdateStatus {
    if matches!(previous, UpdateStatus::Downloading { .. }) { return previous.clone(); }
    match result {
        Ok(Some((version, notes))) => UpdateStatus::Available { version, notes },
        Ok(None) => UpdateStatus::UpToDate,
        Err(message) if manual => UpdateStatus::Failed { message: format!("检查更新失败：{message}"), version: None },
        Err(_) => previous.clone(),
    }
}

/// Whole percent downloaded, when the size is known.
pub fn percent(done: u64, total: Option<u64>) -> Option<u8> {
    match total { Some(total) if total > 0 => Some((done.min(total) * 100 / total) as u8), _ => None }
}

/// Progress moves in steps of 5 %, so the tray menu isn't rebuilt for every chunk.
pub fn worth_showing(shown: Option<u8>, now: Option<u8>) -> bool {
    match (shown, now) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(shown), Some(now)) => now >= shown.saturating_add(5) || (now == 100 && shown != 100),
    }
}
```

- [ ] **Step 3：** 改设置和视图：
  - `config.rs`：加 `pub check_updates: bool`，默认值 `true`。测试 `serde_json::from_str::<Config>(r#"{"version":1}"#)` 读出 `check_updates == true`。
  - `view.rs`：`View` 加 `app_version: String`、`update: UpdateStatus`、`check_updates: bool`。`build_view` 从 cfg 填 `check_updates`，另两个字段先填默认值。
  - `state.rs`：`AppState` 加 `app_version: String` 和 `updates: UpdateState`，`view()` 里把这两项填进去。
- [ ] **Step 4：** `cargo test` 通过。

### Task 4: 接入 tauri-plugin-updater

**Files:** Create `src-tauri/src/app/updater.rs`；Modify `Cargo.toml`、`lib.rs`、`app/mod.rs`、`commands.rs`、`app/tray.rs`

- [ ] **Step 1：** `Cargo.toml` 加依赖 `tauri-plugin-updater = "2"`，`lib.rs` 加 `.plugin(tauri_plugin_updater::Builder::new().build())`。
- [ ] **Step 2：** 写 `app/updater.rs`：

```rust
//! Checking for, and installing, new versions published on GitHub Releases.
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use crate::app::{actions, state::AppState};
use crate::updates::{self, UpdateStatus};

pub const RELEASES_URL: &str = "https://github.com/shulkx/monitor-switch/releases/latest";
const FIRST_CHECK: Duration = Duration::from_secs(30);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Default)]
pub struct UpdateState {
    pub status: Mutex<UpdateStatus>,
    /// What the last successful check found, ready to install.
    pending: Mutex<Option<Update>>,
}

fn set_status(app: &AppHandle, status: UpdateStatus) {
    *app.state::<AppState>().updates.status.lock().unwrap() = status;
    actions::publish_view(app);
}

/// Check 30 s after start, then every 6 h, while "自动检查更新" is on.
pub fn start_background_checks(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(FIRST_CHECK);
        loop {
            if app.state::<AppState>().config.lock().unwrap().check_updates {
                tauri::async_runtime::block_on(check(&app, false));
            }
            thread::sleep(CHECK_EVERY);
        }
    });
}

/// The installer ends this process with exit(0); take the tray icon down first, on the UI
/// thread that owns it, or a dead icon stays in the tray until the mouse passes over it.
fn updater(app: &AppHandle) -> tauri_plugin_updater::Result<Updater> {
    let handle = app.clone();
    app.updater_builder()
        .on_before_exit(move || {
            let (done, wait) = mpsc::channel();
            let app = handle.clone();
            if handle.run_on_main_thread(move || { app.cleanup_before_exit(); let _ = done.send(()); }).is_ok() {
                let _ = wait.recv_timeout(Duration::from_secs(2));
            }
        })
        .build()
}

pub async fn check(app: &AppHandle, manual: bool) {
    let state = app.state::<AppState>();
    if matches!(*state.updates.status.lock().unwrap(), UpdateStatus::Downloading { .. }) { return; }
    if manual { set_status(app, UpdateStatus::Checking); }
    let result = match updater(app) {
        Ok(updater) => updater.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let found = result.map(|update| {
        let info = update.as_ref().map(|u| (u.version.clone(), u.body.clone().filter(|b| !b.trim().is_empty())));
        *state.updates.pending.lock().unwrap() = update;
        info
    });
    let previous = state.updates.status.lock().unwrap().clone();
    set_status(app, updates::after_check(&previous, found, manual));
}

/// Download, verify and run the installer. On success the installer ends this process.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let pending = app.state::<AppState>().updates.pending.lock().unwrap().clone();
    let Some(update) = pending else { return Err("没有可安装的新版本，先检查一下更新".into()) };
    let version = update.version.clone();
    set_status(app, UpdateStatus::Downloading { version: version.clone(), percent: None });
    let (handle, shown_version) = (app.clone(), version.clone());
    let (mut done, mut shown) = (0u64, None);
    let result = update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let now = updates::percent(done, total);
                if updates::worth_showing(shown, now) {
                    shown = now;
                    set_status(&handle, UpdateStatus::Downloading { version: shown_version.clone(), percent: now });
                }
            },
            || {},
        )
        .await;
    result.map_err(|e| {
        let message = format!("更新失败：{e}");
        set_status(app, UpdateStatus::Failed { message: message.clone(), version: Some(version) });
        message
    })
}
```

- [ ] **Step 3：** 接入其他地方：
  - **`app/mod.rs`**：`pub mod updater;`；`AppState` 初始化时填 `app_version: app.package_info().version.to_string()` 和 `updates: UpdateState::default()`；setup 最后调用 `updater::start_background_checks(app)`。
  - **`commands.rs`**：新增三个命令：
    - `check_for_updates`（async）调用 `updater::check(&app, true)`；
    - `install_update`（async）返回 `updater::install(&app)`；
    - `open_release_page` 调用 `platform::open_url(updater::RELEASES_URL)`。
  - **`set_option`**：支持 `"checkUpdates"`。
  - **`lib.rs`**：注册这三个命令。
  - **`tray.rs`**：`build_menu` 在"设置"上方加菜单项：
    - `Available` 状态：`MenuItem::with_id("update", "更新到 {version}")`；
    - `Downloading` 状态：一个不可点的"正在下载更新…"。

    `on_menu` 收到 `"update"` 时执行 `tauri::async_runtime::spawn(async move { let _ = updater::install(&app).await; })`。
- [ ] **Step 4：** `cargo test`、`cargo build` 通过后提交：`feat: update reminders from GitHub Releases`。

### Task 5: 页面

**Files:** Modify `src/shared/api.ts`、`src/flyout/main.ts`、`src/flyout/flyout.css`、`src/settings/main.ts`、`src/settings/settings.css`

- [ ] **Step 1：`api.ts`**
  - 新增 `UpdateStatus` 联合类型，各状态的 `state` 字段分别是 `"idle" | "checking" | "upToDate" | "available" | "downloading" | "failed"`。
  - `View` 加 `appVersion`、`update`、`checkUpdates` 三个字段。
  - `OptionName` 加 `"checkUpdates"`。
  - `api` 加 `checkForUpdates`、`installUpdate`、`openReleasePage`。
- [ ] **Step 2：面板**：`updateBar(v)` 渲染面板底部的提示条，两种渲染分支（有显示器、没显示器）都加上：
  - `available`：显示"新版本 x"和一个"更新"按钮（`data-install`）；
  - `downloading`：显示"正在下载 x · n%"；
  - `failed` 且带版本号：显示错误信息和"去下载页"按钮（`data-release`）。

  在 `afterRender` 里绑定这两个按钮。
- [ ] **Step 3：设置页**：在"常规"后面加 `aboutSection(v)`：
  - 一行显示 `MonitorSwitch {appVersion}`，下方小字是状态说明。
  - 右侧按钮随状态变化：检查更新、更新到 x、去下载页。正在检查或下载时不显示按钮。
  - 下面是"自动检查更新"开关。
  - 有更新说明时，在卡片下方显示，保留换行（`white-space: pre-line`）。
  - 在 `bind` 里绑定 `data-check`、`data-install`、`data-release`。
- [ ] **Step 4：** `npm run build` 通过后提交：`feat: update bar in the panel and an About section in settings`。

### Task 6: 验签脚本

**Files:** Create `scripts/minisign.mjs`、`scripts/minisign.test.mjs`；Modify `package.json`（`"test:scripts": "node --test scripts/"`）

- [ ] **Step 1：** 生成测试样例：
  1. 用临时密钥 `npx tauri signer generate -w <tmp>/k --ci` 生成两把密钥；
  2. 用 `npx tauri signer sign -f <tmp>/k <tmp>/hello.txt` 对一个小文件签名；
  3. 把公钥、`.sig` 内容和文件内容作为常量写进测试。
- [ ] **Step 2：** 测试四种情况：签名正确时通过；文件被改、签名被改、换另一把公钥时都会抛错。
- [ ] **Step 3：** 实现 `verify(data, signatureBase64, publicKeyBase64)`：
  - 签名格式：Ed25519；算法标记为 `ED` 时先对文件做 BLAKE2b-512 预哈希；
  - 先核对 key id，再验文件签名，再验 trusted comment 的全局签名；
  - 不通过时抛出带原因的错误。
- [ ] **Step 4：** `npm run test:scripts` 通过后提交。

### Task 7: 密钥、配置、发布脚本

**Files:** Modify `tauri.conf.json`、`package.json`；Create `src-tauri/release.conf.json`、`scripts/release.mjs`

- [ ] **Step 1：** 生成正式密钥，文件已存在时不覆盖：`npx tauri signer generate -w "%USERPROFILE%\.tauri\monitor-switch.key" --ci`。
- [ ] **Step 2：** `tauri.conf.json` 加：

```json
"plugins": {
  "updater": {
    "pubkey": "<monitor-switch.key.pub 的内容>",
    "endpoints": ["https://github.com/shulkx/monitor-switch/releases/latest/download/latest.json"],
    "windows": { "installMode": "passive" }
  }
}
```

  再新建 `src-tauri/release.conf.json`：`{ "bundle": { "createUpdaterArtifacts": true } }`。
- [ ] **Step 3：** `scripts/release.mjs`，用法 `npm run release -- --notes <file> [--publish]`：
  1. 三处版本号一致；
  2. 读出私钥：环境变量 `TAURI_SIGNING_PRIVATE_KEY` 优先，否则读 `~/.tauri/monitor-switch.key` 的内容；
  3. 执行 `npm run tauri -- build --config src-tauri/release.conf.json`，带上私钥和空密码两个环境变量；
  4. 用 `minisign.verify` 和 `tauri.conf.json` 里的公钥校验 `.sig`；
  5. 写 `latest.json`：包含 `version`、`notes`、`pub_date`，以及 `platforms["windows-x86_64"]` 下的 `signature` 和下载地址 `releases/download/v{version}/MonitorSwitch_{version}_x64-setup.exe`；
  6. 打印 SHA-256；
  7. 只有带 `--publish` 时才执行 `gh release create v{version} <exe> <latest.json> --title "MonitorSwitch {version}" --notes-file <notes>`。
- [ ] **Step 4：** 提交：`build: signed release script and update feed`。

### Task 8: 验证

- [ ] **自动化**：`cargo test`、`npm run build`、`npm run test:scripts` 全部通过。
- [ ] **实机，用临时标识运行开发版**：
  1. 启动 `npm run tauri dev -- --config <scratch>/test.conf.json`。这个配置改 `identifier`、把更新地址指向 `http://localhost:4321/latest.json`，并打开 `dangerousInsecureTransportProtocol`。
  2. 在溢出菜单里点测试实例的图标，截图确认面板停在溢出菜单上方、水平对准图标。
  3. 本地 `latest.json` 写成版本 9.9.9，30 秒后截图确认面板底部的提示条和右键菜单项。
  4. 打开设置，确认"关于"区。依次把本地版本改成当前版本、停掉服务器，各点一次"检查更新"，确认显示"已是最新"和失败原因。
- [ ] **发布构建**：`npm run release -- --notes <notes>`（不带 `--publish`），确认验签通过，`latest.json` 内容正确。

### Task 9: 版本号和文档

- [ ] 版本号改为 0.1.2，涉及 `tauri.conf.json`、`Cargo.toml`、`package.json`、`Cargo.lock`。
- [ ] README：
  - 功能列表加"更新提醒"；
  - 常见问题加"怎么更新"；
  - 开发部分加"发布"小节，写明密钥位置和备份提醒。
- [ ] 提交 `release: 0.1.2`。发布前先问用户。
