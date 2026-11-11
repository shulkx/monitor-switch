//! Owns the DDC backend on one worker thread so monitor commands never overlap (spec 5.1, 5.2).

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;

use crate::ddc::backend::DisplayBackend;
use crate::ddc::caps;
use crate::local_port::guess_local_input;
use crate::model::{ConnectionKind, MonitorInfo, MonitorKey};

pub const NO_RESPONSE: &str = "显示器没响应，检查显示器菜单里的 DDC/CI 是否开启";
pub const NOT_APPLIED: &str = "切换可能没有生效";

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorState {
    pub key: MonitorKey,
    pub label: String,
    pub connection: ConnectionKind,
    pub inputs: Vec<u16>,
    pub current: Option<u16>,
    pub local_input: Option<u16>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeCause {
    /// This app switched it.
    User,
    /// Changed elsewhere: the monitor's buttons, another computer, auto input select.
    External,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceEvent {
    Snapshot(Vec<MonitorState>),
    InputChanged {
        key: MonitorKey,
        from: Option<u16>,
        to: u16,
        cause: ChangeCause,
        local_input: Option<u16>,
    },
    SwitchFailed {
        key: MonitorKey,
        value: u16,
        reason: String,
    },
}

pub enum CapsDecision {
    Cached(String),
    Read,
    Skip,
}

/// Settings the service needs. Implemented over the config file by the app.
pub trait MonitorPrefs: Send + Sync {
    fn caps_decision(&self, key: &MonitorKey) -> CapsDecision;
    /// Called right before reading capabilities; must be persisted immediately.
    fn begin_caps_read(&self, key: &MonitorKey, label: &str);
    fn finish_caps_read(&self, key: &MonitorKey, raw: Option<String>);
    fn local_override(&self, key: &MonitorKey) -> Option<u16>;
}

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    pub poll: Duration,
    pub verify_delay: Duration,
    pub retry_delay: Duration,
    pub attempts: u32,
    /// Enumerate monitors again every N polls, to notice plugged / unplugged monitors.
    pub relist_every: u32,
    pub poll_failures_before_error: u32,
}

impl Timing {
    pub fn production() -> Self {
        Timing {
            poll: Duration::from_secs(4),
            verify_delay: Duration::from_millis(1500),
            retry_delay: Duration::from_millis(50),
            attempts: 3,
            relist_every: 5,
            poll_failures_before_error: 3,
        }
    }
}

enum Command {
    Refresh,
    Switch { key: MonitorKey, value: u16 },
    Shutdown,
}

pub struct MonitorService {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<Vec<MonitorState>>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MonitorService {
    pub fn start<F>(
        backend: Box<dyn DisplayBackend>,
        prefs: Arc<dyn MonitorPrefs>,
        is_laptop: bool,
        timing: Timing,
        on_event: F,
    ) -> Self
    where
        F: Fn(ServiceEvent) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Vec::new()));
        let mut worker = Worker {
            backend,
            prefs,
            is_laptop,
            timing,
            on_event: Box::new(on_event),
            snapshot: snapshot.clone(),
            monitors: Vec::new(),
            poll_failures: HashMap::new(),
            polls_since_list: 0,
        };
        let handle = thread::spawn(move || worker.run(rx));
        MonitorService { tx, snapshot, handle: Some(handle) }
    }

    /// Enumerate monitors and read everything again.
    pub fn refresh(&self) {
        let _ = self.tx.send(Command::Refresh);
    }

    pub fn switch(&self, key: MonitorKey, value: u16) {
        let _ = self.tx.send(Command::Switch { key, value });
    }

    pub fn snapshot(&self) -> Vec<MonitorState> {
        self.snapshot.lock().unwrap().clone()
    }
}

impl Drop for MonitorService {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct Worker {
    backend: Box<dyn DisplayBackend>,
    prefs: Arc<dyn MonitorPrefs>,
    is_laptop: bool,
    timing: Timing,
    on_event: Box<dyn Fn(ServiceEvent) + Send>,
    snapshot: Arc<Mutex<Vec<MonitorState>>>,
    monitors: Vec<MonitorState>,
    poll_failures: HashMap<MonitorKey, u32>,
    polls_since_list: u32,
}

impl Worker {
    fn run(&mut self, rx: Receiver<Command>) {
        self.relist();
        loop {
            match rx.recv_timeout(self.timing.poll) {
                Ok(Command::Refresh) => self.relist(),
                Ok(Command::Switch { key, value }) => self.switch(&key, value),
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => self.poll(),
            }
        }
    }

    fn relist(&mut self) {
        self.polls_since_list = 0;
        let infos = self.backend.list();
        let mut next = Vec::with_capacity(infos.len());
        for info in infos {
            let inputs = self.inputs_for(&info);
            let previous = self.monitors.iter().find(|m| m.key == info.key).and_then(|m| m.current);
            let (current, error) = match self.read_with_retry(&info.key) {
                Ok(v) => (Some(v), None),
                Err(_) => (previous, Some(NO_RESPONSE.to_string())),
            };
            let local_input = self
                .prefs
                .local_override(&info.key)
                .or_else(|| guess_local_input(info.connection, self.is_laptop, &inputs));
            next.push(MonitorState {
                key: info.key,
                label: info.label,
                connection: info.connection,
                inputs,
                current,
                local_input,
                error,
            });
        }
        self.monitors = next;
        self.publish();
    }

    fn inputs_for(&mut self, info: &MonitorInfo) -> Vec<u16> {
        let raw = match self.prefs.caps_decision(&info.key) {
            CapsDecision::Cached(raw) => Some(raw),
            CapsDecision::Skip => None,
            CapsDecision::Read => {
                self.prefs.begin_caps_read(&info.key, &info.label);
                let raw = self.backend.capabilities(&info.key).ok();
                self.prefs.finish_caps_read(&info.key, raw.clone());
                raw
            }
        };
        raw.map(|r| caps::parse(&r).inputs).unwrap_or_default()
    }

    fn read_once(&mut self, key: &MonitorKey) -> Result<u16, String> {
        // Some monitors pad the value with extra bytes; only the low byte is the input.
        self.backend.read_input(key).map(|v| v & 0x00FF)
    }

    fn read_with_retry(&mut self, key: &MonitorKey) -> Result<u16, String> {
        let mut last = Err(String::new());
        for attempt in 0..self.timing.attempts {
            last = self.read_once(key);
            if last.is_ok() {
                break;
            }
            if attempt + 1 < self.timing.attempts {
                thread::sleep(self.timing.retry_delay);
            }
        }
        last
    }

    fn poll(&mut self) {
        self.polls_since_list += 1;
        if self.monitors.is_empty() || self.polls_since_list >= self.timing.relist_every {
            self.relist();
            return;
        }
        let mut changed = false;
        let mut events = Vec::new();
        for idx in 0..self.monitors.len() {
            let key = self.monitors[idx].key.clone();
            match self.read_once(&key) {
                Ok(v) => {
                    self.poll_failures.remove(&key);
                    let m = &mut self.monitors[idx];
                    if m.error.take().is_some() {
                        changed = true;
                    }
                    if m.current != Some(v) {
                        if m.current.is_some() {
                            events.push(ServiceEvent::InputChanged {
                                key: key.clone(),
                                from: m.current,
                                to: v,
                                cause: ChangeCause::External,
                                local_input: m.local_input,
                            });
                        }
                        m.current = Some(v);
                        changed = true;
                    }
                }
                Err(_) => {
                    let failures = self.poll_failures.entry(key).or_insert(0);
                    *failures += 1;
                    if *failures >= self.timing.poll_failures_before_error && self.monitors[idx].error.is_none() {
                        self.monitors[idx].error = Some(NO_RESPONSE.to_string());
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.publish();
        }
        for event in events {
            (self.on_event)(event);
        }
    }

    fn switch(&mut self, key: &MonitorKey, value: u16) {
        let mut sent = false;
        for attempt in 0..self.timing.attempts {
            if self.backend.set_input(key, value).is_ok() {
                sent = true;
                break;
            }
            if attempt + 1 < self.timing.attempts {
                thread::sleep(self.timing.retry_delay);
            }
        }
        if !sent {
            (self.on_event)(ServiceEvent::SwitchFailed { key: key.clone(), value, reason: NO_RESPONSE.to_string() });
            return;
        }
        thread::sleep(self.timing.verify_delay);
        let read = self.read_with_retry(key);
        let Some(idx) = self.monitors.iter().position(|m| &m.key == key) else {
            return;
        };
        let from = self.monitors[idx].current;
        let local_input = self.monitors[idx].local_input;
        let event = match read {
            Ok(v) if v != value => {
                self.monitors[idx].current = Some(v);
                ServiceEvent::SwitchFailed { key: key.clone(), value, reason: NOT_APPLIED.to_string() }
            }
            // The expected value, or no answer: many monitors go quiet for a moment while they
            // lock on to the new signal. The command was accepted, so treat it as done.
            _ => {
                self.monitors[idx].current = Some(value);
                self.monitors[idx].error = None;
                ServiceEvent::InputChanged { key: key.clone(), from, to: value, cause: ChangeCause::User, local_input }
            }
        };
        self.publish();
        (self.on_event)(event);
    }

    /// Store the state for `snapshot()` first, then notify, so listeners always read fresh state.
    fn publish(&mut self) {
        *self.snapshot.lock().unwrap() = self.monitors.clone();
        (self.on_event)(ServiceEvent::Snapshot(self.monitors.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ddc::caps::DELL_STYLE_CAPS;
    use crate::ddc::fake::{dell, FakeBackend};
    use std::time::Instant;

    fn timing() -> Timing {
        Timing {
            poll: Duration::from_millis(30),
            verify_delay: Duration::from_millis(10),
            retry_delay: Duration::from_millis(1),
            attempts: 3,
            relist_every: 1000,
            poll_failures_before_error: 2,
        }
    }

    #[derive(Default)]
    struct TestPrefs {
        cached: Option<String>,
        override_input: Option<u16>,
        log: Mutex<Vec<String>>,
    }

    impl MonitorPrefs for TestPrefs {
        fn caps_decision(&self, _key: &MonitorKey) -> CapsDecision {
            match &self.cached {
                Some(raw) => CapsDecision::Cached(raw.clone()),
                None => CapsDecision::Read,
            }
        }
        fn begin_caps_read(&self, key: &MonitorKey, _label: &str) {
            self.log.lock().unwrap().push(format!("begin {}", key.0));
        }
        fn finish_caps_read(&self, key: &MonitorKey, raw: Option<String>) {
            self.log.lock().unwrap().push(format!("finish {} {}", key.0, raw.is_some()));
        }
        fn local_override(&self, _key: &MonitorKey) -> Option<u16> {
            self.override_input
        }
    }

    fn fake_with_dell(current: u16) -> FakeBackend {
        let fake = FakeBackend::default();
        {
            let mut s = fake.0.lock().unwrap();
            s.monitors.push(dell());
            s.inputs.insert(dell().key, current);
            s.caps.insert(dell().key, DELL_STYLE_CAPS.to_string());
        }
        fake
    }

    fn start(fake: &FakeBackend, prefs: TestPrefs) -> (MonitorService, Receiver<ServiceEvent>, Arc<TestPrefs>) {
        let prefs = Arc::new(prefs);
        let (tx, rx) = mpsc::channel();
        let service = MonitorService::start(Box::new(fake.clone()), prefs.clone(), false, timing(), move |e| {
            let _ = tx.send(e);
        });
        (service, rx, prefs)
    }

    fn wait_for(rx: &Receiver<ServiceEvent>, pred: impl Fn(&ServiceEvent) -> bool) -> ServiceEvent {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(e) if pred(&e) => return e,
                Ok(_) => continue,
                Err(_) => panic!("timed out waiting for an event"),
            }
        }
    }

    fn first_snapshot(rx: &Receiver<ServiceEvent>) -> Vec<MonitorState> {
        match wait_for(rx, |e| matches!(e, ServiceEvent::Snapshot(_))) {
            ServiceEvent::Snapshot(states) => states,
            _ => unreachable!(),
        }
    }

    #[test]
    fn first_snapshot_describes_the_monitor() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, _) = start(&fake, TestPrefs::default());
        let states = first_snapshot(&rx);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].current, Some(0x0F));
        assert_eq!(states[0].inputs, vec![0x0F, 0x11, 0x1B]);
        assert_eq!(states[0].local_input, Some(0x0F));
        assert_eq!(states[0].error, None);
    }

    #[test]
    fn reads_capabilities_and_reports_progress() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, prefs) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        assert_eq!(fake.0.lock().unwrap().caps_reads, 1);
        assert_eq!(
            *prefs.log.lock().unwrap(),
            vec!["begin DEL426A#UID1".to_string(), "finish DEL426A#UID1 true".to_string()]
        );
    }

    #[test]
    fn uses_cached_capabilities_without_asking_the_monitor() {
        let fake = fake_with_dell(0x0F);
        let prefs = TestPrefs { cached: Some(DELL_STYLE_CAPS.to_string()), ..Default::default() };
        let (_svc, rx, _) = start(&fake, prefs);
        assert_eq!(first_snapshot(&rx)[0].inputs, vec![0x0F, 0x11, 0x1B]);
        assert_eq!(fake.0.lock().unwrap().caps_reads, 0);
    }

    #[test]
    fn local_override_wins_over_the_guess() {
        let fake = fake_with_dell(0x0F);
        let prefs = TestPrefs { override_input: Some(0x11), ..Default::default() };
        let (_svc, rx, _) = start(&fake, prefs);
        assert_eq!(first_snapshot(&rx)[0].local_input, Some(0x11));
    }

    #[test]
    fn switch_changes_input_and_reports_a_user_change() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        svc.switch(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::InputChanged { .. }));
        assert_eq!(
            event,
            ServiceEvent::InputChanged {
                key: dell().key,
                from: Some(0x0F),
                to: 0x1B,
                cause: ChangeCause::User,
                local_input: Some(0x0F),
            }
        );
        assert_eq!(fake.0.lock().unwrap().set_log, vec![(dell().key, 0x1B)]);
        assert_eq!(svc.snapshot()[0].current, Some(0x1B));
    }

    #[test]
    fn switch_retries_transient_failures() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().fail_next_sets = 2;
        svc.switch(dell().key, 0x1B);
        wait_for(&rx, |e| matches!(e, ServiceEvent::InputChanged { cause: ChangeCause::User, .. }));
        assert_eq!(fake.0.lock().unwrap().set_log.len(), 1);
    }

    #[test]
    fn switch_that_never_goes_through_is_reported() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().fail_next_sets = 3;
        svc.switch(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::SwitchFailed { .. }));
        assert_eq!(event, ServiceEvent::SwitchFailed { key: dell().key, value: 0x1B, reason: NO_RESPONSE.into() });
    }

    #[test]
    fn switch_the_monitor_ignores_is_reported() {
        let fake = fake_with_dell(0x0F);
        let (svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().ignore_sets = true;
        svc.switch(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::SwitchFailed { .. }));
        assert_eq!(event, ServiceEvent::SwitchFailed { key: dell().key, value: 0x1B, reason: NOT_APPLIED.into() });
    }

    #[test]
    fn polling_notices_changes_made_elsewhere() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().inputs.insert(dell().key, 0x1B);
        let event = wait_for(&rx, |e| matches!(e, ServiceEvent::InputChanged { .. }));
        assert_eq!(
            event,
            ServiceEvent::InputChanged {
                key: dell().key,
                from: Some(0x0F),
                to: 0x1B,
                cause: ChangeCause::External,
                local_input: Some(0x0F),
            }
        );
    }

    #[test]
    fn repeated_read_failures_mark_the_monitor_unresponsive() {
        let fake = fake_with_dell(0x0F);
        let (_svc, rx, _) = start(&fake, TestPrefs::default());
        first_snapshot(&rx);
        fake.0.lock().unwrap().fail_reads = true;
        wait_for(&rx, |e| match e {
            ServiceEvent::Snapshot(states) => states[0].error.as_deref() == Some(NO_RESPONSE),
            _ => false,
        });
    }
}
