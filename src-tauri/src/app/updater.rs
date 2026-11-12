//! Checking for, and installing, new versions published on GitHub Releases.
//! The user is only reminded; nothing is downloaded until they click "更新".

use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use crate::app::{actions, state::AppState};
use crate::updates::{self, UpdateStatus};

/// Where to download by hand when updating in place fails.
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

/// The installer ends this process with exit(0). Take the tray icon down first, on the UI thread
/// that owns it, or a dead icon stays in the tray until the mouse passes over it.
fn updater(app: &AppHandle) -> tauri_plugin_updater::Result<Updater> {
    let handle = app.clone();
    app.updater_builder()
        .on_before_exit(move || {
            let (done, wait) = mpsc::channel();
            let app = handle.clone();
            let posted = handle.run_on_main_thread(move || {
                app.cleanup_before_exit();
                let _ = done.send(());
            });
            if posted.is_ok() {
                let _ = wait.recv_timeout(Duration::from_secs(2));
            }
        })
        .build()
}

/// Look for a newer version. `manual`: the user pressed "检查更新" and wants to hear back.
pub async fn check(app: &AppHandle, manual: bool) {
    let state = app.state::<AppState>();
    if matches!(*state.updates.status.lock().unwrap(), UpdateStatus::Downloading { .. }) {
        return;
    }
    if manual {
        set_status(app, UpdateStatus::Checking);
    }
    let result = match updater(app) {
        Ok(updater) => updater.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let found = result.map(|update| {
        let info = update
            .as_ref()
            .map(|u| (u.version.clone(), u.body.clone().filter(|notes| !notes.trim().is_empty())));
        *state.updates.pending.lock().unwrap() = update;
        info
    });
    let previous = state.updates.status.lock().unwrap().clone();
    set_status(app, updates::after_check(&previous, found, manual));
}

/// Download the update the last check found, verify its signature and run the installer, which
/// ends this process and opens the new version when it is done.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let pending = app.state::<AppState>().updates.pending.lock().unwrap().clone();
    let Some(update) = pending else {
        return Err("没有可安装的新版本，先检查一下更新".into());
    };
    if matches!(*app.state::<AppState>().updates.status.lock().unwrap(), UpdateStatus::Downloading { .. }) {
        return Ok(());
    }
    let version = update.version.clone();
    set_status(app, UpdateStatus::Downloading { version: version.clone(), percent: None });
    let handle = app.clone();
    let shown_version = version.clone();
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
