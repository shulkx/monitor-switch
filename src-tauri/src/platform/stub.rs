use crate::ddc::backend::DisplayBackend;
use crate::model::{MonitorInfo, MonitorKey};

const UNSUPPORTED: &str = "this platform is not supported yet";

#[derive(Default)]
pub struct StubBackend;

impl StubBackend {
    pub fn new() -> Self {
        StubBackend
    }
}

impl DisplayBackend for StubBackend {
    fn list(&mut self) -> Vec<MonitorInfo> {
        Vec::new()
    }
    fn read_input(&mut self, _key: &MonitorKey) -> Result<u16, String> {
        Err(UNSUPPORTED.into())
    }
    fn set_input(&mut self, _key: &MonitorKey, _value: u16) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    fn capabilities(&mut self, _key: &MonitorKey) -> Result<String, String> {
        Err(UNSUPPORTED.into())
    }
}

pub fn is_laptop() -> bool {
    false
}
