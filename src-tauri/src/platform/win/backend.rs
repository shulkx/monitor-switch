//! DDC/CI over dxva2, matched to connectors from QueryDisplayConfig by monitor device path.

use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use super::ffi::*;
use super::topology;
use crate::ddc::backend::DisplayBackend;
use crate::ddc::caps::VCP_INPUT_SELECT;
use crate::model::{ConnectionKind, MonitorInfo, MonitorKey};

/// `\\?\display#del426a#5&33621b4e&0&uid4358#{guid}` → `DEL426A#5&33621B4E&0&UID4358`.
pub fn key_from_device_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('#').collect();
    if parts.len() < 3 || parts[1].is_empty() || parts[2].is_empty() {
        return None;
    }
    Some(format!("{}#{}", parts[1], parts[2]).to_ascii_uppercase())
}

/// Registry key holding a monitor's EDID, from its interface path:
/// `\\?\display#del426a#5&33621b4e&0&uid4358#{guid}` →
/// `SYSTEM\CurrentControlSet\Enum\DISPLAY\del426a\5&33621b4e&0&uid4358\Device Parameters`.
pub fn edid_registry_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('#').collect();
    if parts.len() < 3 || parts[1].is_empty() || parts[2].is_empty() {
        return None;
    }
    Some(format!(r"SYSTEM\CurrentControlSet\Enum\DISPLAY\{}\{}\Device Parameters", parts[1], parts[2]))
}

/// "DEL-426A-ABC1234": manufacturer, product code and serial from an EDID block (spec 5.4).
/// The same physical monitor gets the same key on every computer it is plugged into.
pub fn key_from_edid(edid: &[u8]) -> Option<String> {
    const HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    if edid.len() < 128 || edid[..8] != HEADER {
        return None;
    }
    let packed = u16::from_be_bytes([edid[8], edid[9]]);
    let letter = |shift: u16| (b'A' + ((packed >> shift) & 0x1F) as u8 - 1) as char;
    let maker: String = [letter(10), letter(5), letter(0)].iter().collect();
    let product = u16::from_le_bytes([edid[10], edid[11]]);
    // A display descriptor with tag 0xFF holds the serial number as text.
    let serial_text = (0..4).map(|i| 54 + i * 18).find_map(|offset| {
        let d = &edid[offset..offset + 18];
        if d[0] != 0 || d[1] != 0 || d[3] != 0xFF {
            return None;
        }
        let text: String = d[5..]
            .iter()
            .take_while(|&&b| b != 0x0A)
            .map(|&b| b as char)
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        (!text.is_empty()).then_some(text)
    });
    let serial_number = u32::from_le_bytes([edid[12], edid[13], edid[14], edid[15]]);
    let serial = serial_text.or_else(|| (serial_number != 0).then(|| serial_number.to_string()));
    Some(match serial {
        Some(serial) => format!("{maker}-{product:04X}-{serial}"),
        None => format!("{maker}-{product:04X}"),
    })
}

fn read_edid(device_path: &str) -> Option<Vec<u8>> {
    let sub_key = to_wide(&edid_registry_path(device_path)?);
    let value = to_wide("EDID");
    let mut size = 0u32;
    unsafe {
        let rc = RegGetValueW(HKEY_LOCAL_MACHINE, sub_key.as_ptr(), value.as_ptr(), RRF_RT_REG_BINARY, null_mut(), null_mut(), &mut size);
        if rc != 0 || size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        let rc = RegGetValueW(HKEY_LOCAL_MACHINE, sub_key.as_ptr(), value.as_ptr(), RRF_RT_REG_BINARY, null_mut(), data.as_mut_ptr(), &mut size);
        if rc != 0 {
            return None;
        }
        data.truncate(size as usize);
        Some(data)
    }
}

/// The driver's description ("Dell U2723QE (DP HDR)") without the connection suffix, or the
/// EDID name when the description is the generic one.
pub fn label_for(description: &str, friendly_name: Option<&str>) -> String {
    let description = description.trim();
    let generic = description.is_empty()
        || description.to_ascii_lowercase().starts_with("generic")
        || description.contains("通用");
    let friendly = friendly_name.map(str::trim).filter(|f| !f.is_empty());
    if generic {
        if let Some(f) = friendly {
            return f.to_string();
        }
    }
    let trimmed = match description.rfind(" (") {
        Some(i) if description.ends_with(')') => &description[..i],
        _ => description,
    };
    if trimmed.is_empty() {
        friendly.unwrap_or("显示器").to_string()
    } else {
        trimmed.to_string()
    }
}

struct Physical {
    info: MonitorInfo,
    handle: isize,
}

pub struct WinBackend {
    monitors: Vec<Physical>,
}

impl WinBackend {
    pub fn new() -> Self {
        WinBackend { monitors: Vec::new() }
    }

    fn release(&mut self) {
        for m in self.monitors.drain(..) {
            unsafe {
                DestroyPhysicalMonitor(m.handle);
            }
        }
    }

    fn handle(&self, key: &MonitorKey) -> Result<isize, String> {
        self.monitors
            .iter()
            .find(|m| &m.info.key == key)
            .map(|m| m.handle)
            .ok_or_else(|| "monitor not found".to_string())
    }
}

impl Default for WinBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WinBackend {
    fn drop(&mut self) {
        self.release();
    }
}

impl DisplayBackend for WinBackend {
    fn list(&mut self) -> Vec<MonitorInfo> {
        self.release();
        let targets = topology::active_targets();
        for hmonitor in enum_monitors() {
            let device = monitor_device_name(hmonitor);
            let paths = active_monitor_paths(&device);
            // The i-th physical monitor of an HMONITOR is the i-th active monitor device on it.
            for (i, (handle, description)) in physical_monitors(hmonitor).into_iter().enumerate() {
                let path = paths.get(i);
                let target = path.and_then(|p| targets.iter().find(|t| &t.device_path == p));
                if target.map(|t| t.internal).unwrap_or(false) {
                    // A laptop's own screen: no DDC/CI input switching.
                    unsafe {
                        DestroyPhysicalMonitor(handle);
                    }
                    continue;
                }
                let mut key = path
                    .and_then(|p| read_edid(p))
                    .and_then(|edid| key_from_edid(&edid))
                    .or_else(|| path.and_then(|p| key_from_device_path(p)))
                    .unwrap_or_else(|| format!("{device}#{i}"));
                // Two identical monitors without serial numbers would otherwise share a key.
                if self.monitors.iter().any(|m| m.info.key.0 == key) {
                    key = format!("{key}#{}", self.monitors.len() + 1);
                }
                let info = MonitorInfo {
                    key: MonitorKey(key),
                    label: label_for(&description, target.map(|t| t.friendly_name.as_str())),
                    connection: target.map(|t| t.connection).unwrap_or(ConnectionKind::Other),
                };
                self.monitors.push(Physical { info, handle });
            }
        }
        self.monitors.iter().map(|m| m.info.clone()).collect()
    }

    fn read_input(&mut self, key: &MonitorKey) -> Result<u16, String> {
        let handle = self.handle(key)?;
        let (mut current, mut maximum) = (0u32, 0u32);
        let ok = unsafe { GetVCPFeatureAndVCPFeatureReply(handle, VCP_INPUT_SELECT, null_mut(), &mut current, &mut maximum) };
        if ok == 0 {
            return Err("DDC/CI read failed".into());
        }
        Ok((current & 0xFFFF) as u16)
    }

    fn set_input(&mut self, key: &MonitorKey, value: u16) -> Result<(), String> {
        let handle = self.handle(key)?;
        if unsafe { SetVCPFeature(handle, VCP_INPUT_SELECT, value as u32) } == 0 {
            return Err("DDC/CI write failed".into());
        }
        Ok(())
    }

    fn capabilities(&mut self, key: &MonitorKey) -> Result<String, String> {
        let handle = self.handle(key)?;
        let mut length = 0u32;
        if unsafe { GetCapabilitiesStringLength(handle, &mut length) } == 0 || length == 0 {
            return Err("capabilities length unavailable".into());
        }
        let mut buffer = vec![0u8; length as usize];
        if unsafe { CapabilitiesRequestAndCapabilitiesReply(handle, buffer.as_mut_ptr(), length) } == 0 {
            return Err("capabilities request failed".into());
        }
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        Ok(String::from_utf8_lossy(&buffer[..end]).into_owned())
    }
}

unsafe extern "system" fn collect_monitor(hmonitor: isize, _hdc: isize, _rect: *mut RECT, data: isize) -> i32 {
    let list = &mut *(data as *mut Vec<isize>);
    list.push(hmonitor);
    1
}

fn enum_monitors() -> Vec<isize> {
    let mut list: Vec<isize> = Vec::new();
    unsafe {
        EnumDisplayMonitors(0, null(), Some(collect_monitor), &mut list as *mut Vec<isize> as isize);
    }
    list
}

fn monitor_device_name(hmonitor: isize) -> String {
    let mut info: MONITORINFOEXW = unsafe { zeroed() };
    info.cb_size = size_of::<MONITORINFOEXW>() as u32;
    if unsafe { GetMonitorInfoW(hmonitor, &mut info) } == 0 {
        return String::new();
    }
    wide_to_string(&info.sz_device)
}

/// Interface paths of the monitor devices that are actually showing this desktop (not just attached).
fn active_monitor_paths(device: &str) -> Vec<String> {
    let device = to_wide(device);
    let mut paths = Vec::new();
    for index in 0u32.. {
        let mut dd: DISPLAY_DEVICEW = unsafe { zeroed() };
        dd.cb = size_of::<DISPLAY_DEVICEW>() as u32;
        if unsafe { EnumDisplayDevicesW(device.as_ptr(), index, &mut dd, EDD_GET_DEVICE_INTERFACE_NAME) } == 0 {
            break;
        }
        if dd.state_flags & DISPLAY_DEVICE_ACTIVE != 0 {
            paths.push(wide_to_string(&dd.device_id).to_ascii_lowercase());
        }
    }
    paths
}

fn physical_monitors(hmonitor: isize) -> Vec<(isize, String)> {
    let mut count = 0u32;
    if unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count) } == 0 || count == 0 {
        return Vec::new();
    }
    let mut list: Vec<PHYSICAL_MONITOR> = vec![unsafe { zeroed() }; count as usize];
    if unsafe { GetPhysicalMonitorsFromHMONITOR(hmonitor, count, list.as_mut_ptr()) } == 0 {
        return Vec::new();
    }
    list.iter().map(|p| (p.handle, wide_to_string(&p.description))).collect()
}

/// A battery means a laptop. 128 = no system battery, 255 = unknown.
pub fn is_laptop() -> bool {
    let mut status: SYSTEM_POWER_STATUS = unsafe { zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return false;
    }
    status.battery_flag != 128 && status.battery_flag != 255
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_keys_from_device_paths() {
        assert_eq!(
            key_from_device_path(r"\\?\display#del426a#5&33621b4e&0&uid4358#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}"),
            Some("DEL426A#5&33621B4E&0&UID4358".to_string())
        );
        assert_eq!(key_from_device_path("garbage"), None);
    }

    /// A minimal EDID: header, "DEL" (0x10AC), product 0x426A, optional serial descriptor.
    fn edid(serial_number: u32, serial_text: Option<&str>) -> Vec<u8> {
        let mut e = vec![0u8; 128];
        e[..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);
        e[8..10].copy_from_slice(&[0x10, 0xAC]);
        e[10..12].copy_from_slice(&[0x6A, 0x42]);
        e[12..16].copy_from_slice(&serial_number.to_le_bytes());
        if let Some(text) = serial_text {
            e[54..59].copy_from_slice(&[0, 0, 0, 0xFF, 0]);
            let mut body = [0x20u8; 13];
            body[..text.len()].copy_from_slice(text.as_bytes());
            body[text.len()] = 0x0A;
            e[59..72].copy_from_slice(&body);
        }
        e
    }

    #[test]
    fn builds_keys_from_edid() {
        assert_eq!(key_from_edid(&edid(0, Some("ABC1234"))), Some("DEL-426A-ABC1234".to_string()));
        assert_eq!(key_from_edid(&edid(1234, None)), Some("DEL-426A-1234".to_string()));
        assert_eq!(key_from_edid(&edid(0, None)), Some("DEL-426A".to_string()));
        assert_eq!(key_from_edid(&[0u8; 128]), None);
        assert_eq!(key_from_edid(&[0u8; 10]), None);
    }

    #[test]
    fn finds_the_edid_registry_key() {
        assert_eq!(
            edid_registry_path(r"\\?\display#del426a#5&33621b4e&0&uid4358#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}").as_deref(),
            Some(r"SYSTEM\CurrentControlSet\Enum\DISPLAY\del426a\5&33621b4e&0&uid4358\Device Parameters")
        );
    }

    #[test]
    fn labels_monitors() {
        assert_eq!(label_for("Dell U2723QE (DP HDR)", Some("DELL U2723QE")), "Dell U2723QE");
        assert_eq!(label_for("Generic PnP Monitor", Some("LG ULTRAFINE")), "LG ULTRAFINE");
        assert_eq!(label_for("通用即插即用监视器", Some("Mi TV")), "Mi TV");
        assert_eq!(label_for("", None), "显示器");
    }

    /// Talks to the real monitors. Run with: cargo test -- --ignored --nocapture lists_real_monitors
    #[test]
    #[ignore]
    fn lists_real_monitors() {
        let mut backend = WinBackend::new();
        let monitors = backend.list();
        for m in &monitors {
            println!("{:?} input={:?}", m, backend.read_input(&m.key));
        }
        assert!(!monitors.is_empty());
    }
}
