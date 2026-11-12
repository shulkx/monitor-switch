//! What the update reminder shows (design 2026-11-12-flyout-anchor-and-updates, section 3).

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
    /// `version` is set when installing that version failed; `None` when checking failed.
    Failed { message: String, version: Option<String> },
}

/// A newer version and its release notes, or `None` when this one is the latest.
pub type Found = Option<(String, Option<String>)>;

/// The state after a check. A failed background check keeps what was shown; a check the user
/// asked for says why it failed. A download in progress is never replaced.
pub fn after_check(previous: &UpdateStatus, result: Result<Found, String>, manual: bool) -> UpdateStatus {
    if matches!(previous, UpdateStatus::Downloading { .. }) {
        return previous.clone();
    }
    match result {
        Ok(Some((version, notes))) => UpdateStatus::Available { version, notes },
        Ok(None) => UpdateStatus::UpToDate,
        Err(message) if manual => UpdateStatus::Failed { message: format!("检查更新失败：{message}"), version: None },
        Err(_) => previous.clone(),
    }
}

/// Whole percent downloaded, when the size is known.
pub fn percent(done: u64, total: Option<u64>) -> Option<u8> {
    match total {
        Some(total) if total > 0 => Some((done.min(total) * 100 / total) as u8),
        _ => None,
    }
}

/// Progress moves in steps of 5 %, so the tray menu isn't rebuilt for every chunk.
pub fn worth_showing(shown: Option<u8>, now: Option<u8>) -> bool {
    match (shown, now) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(shown), Some(now)) => now >= shown.saturating_add(5) || (now == 100 && shown != 100),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn available(v: &str) -> UpdateStatus {
        UpdateStatus::Available { version: v.into(), notes: None }
    }

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
        assert!(!worth_showing(Some(100), Some(100)));
        assert!(!worth_showing(None, None));
    }

    #[test]
    fn serializes_for_the_pages() {
        let json = serde_json::to_value(UpdateStatus::Downloading { version: "0.1.3".into(), percent: Some(5) }).unwrap();
        assert_eq!(json, serde_json::json!({ "state": "downloading", "version": "0.1.3", "percent": 5 }));
        assert_eq!(serde_json::to_value(UpdateStatus::UpToDate).unwrap(), serde_json::json!({ "state": "upToDate" }));
    }
}
