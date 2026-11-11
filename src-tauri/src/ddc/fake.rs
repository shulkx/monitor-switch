//! In-memory backend for tests. Tests keep a clone and inspect or change the shared state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::backend::DisplayBackend;
use crate::model::{ConnectionKind, MonitorInfo, MonitorKey};

#[derive(Default)]
pub struct FakeState {
    pub monitors: Vec<MonitorInfo>,
    pub inputs: HashMap<MonitorKey, u16>,
    pub caps: HashMap<MonitorKey, String>,
    pub caps_reads: u32,
    pub set_log: Vec<(MonitorKey, u16)>,
    /// The next N set_input calls fail.
    pub fail_next_sets: u32,
    /// set_input succeeds but the input does not change.
    pub ignore_sets: bool,
    pub fail_reads: bool,
}

#[derive(Clone, Default)]
pub struct FakeBackend(pub Arc<Mutex<FakeState>>);

impl DisplayBackend for FakeBackend {
    fn list(&mut self) -> Vec<MonitorInfo> {
        self.0.lock().unwrap().monitors.clone()
    }

    fn read_input(&mut self, key: &MonitorKey) -> Result<u16, String> {
        let s = self.0.lock().unwrap();
        if s.fail_reads {
            return Err("read failed".into());
        }
        s.inputs.get(key).copied().ok_or_else(|| "no such monitor".into())
    }

    fn set_input(&mut self, key: &MonitorKey, value: u16) -> Result<(), String> {
        let mut s = self.0.lock().unwrap();
        if s.fail_next_sets > 0 {
            s.fail_next_sets -= 1;
            return Err("set failed".into());
        }
        s.set_log.push((key.clone(), value));
        if !s.ignore_sets {
            s.inputs.insert(key.clone(), value);
        }
        Ok(())
    }

    fn capabilities(&mut self, key: &MonitorKey) -> Result<String, String> {
        let mut s = self.0.lock().unwrap();
        s.caps_reads += 1;
        s.caps.get(key).cloned().ok_or_else(|| "no capabilities".into())
    }
}

pub fn dell() -> MonitorInfo {
    MonitorInfo {
        key: MonitorKey("DEL426A#UID1".into()),
        label: "Dell U2723QE".into(),
        connection: ConnectionKind::DisplayPort,
    }
}
