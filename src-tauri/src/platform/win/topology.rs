//! Which connector each active monitor uses, from QueryDisplayConfig.

use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use super::ffi::*;
use crate::model::ConnectionKind;

#[derive(Clone, Debug)]
pub struct Target {
    /// Lower-case monitor interface path, e.g. `\\?\display#del426a#5&33621b4e&0&uid4358#{e6f07b5f-...}`.
    pub device_path: String,
    pub friendly_name: String,
    pub connection: ConnectionKind,
}

/// DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY → ConnectionKind.
/// 19 is DISPLAYPORT_USB_TUNNEL on recent Windows 11 builds; older SDKs do not define it.
pub fn connection_from_technology(technology: u32) -> ConnectionKind {
    match technology {
        5 => ConnectionKind::Hdmi,
        10 | 11 => ConnectionKind::DisplayPort,
        19 => ConnectionKind::UsbTunnel,
        _ => ConnectionKind::Other,
    }
}

pub fn active_targets() -> Vec<Target> {
    unsafe {
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO>;
        let mut path_count = 0u32;
        loop {
            let mut mode_count = 0u32;
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count) != 0 {
                return Vec::new();
            }
            paths = vec![zeroed(); path_count as usize];
            let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = vec![zeroed(); mode_count as usize];
            let rc = QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                null_mut(),
            );
            if rc == ERROR_INSUFFICIENT_BUFFER {
                continue; // the display setup changed between the two calls
            }
            if rc != 0 {
                return Vec::new();
            }
            break;
        }
        paths.truncate(path_count as usize);
        paths
            .iter()
            .filter_map(|p| target_name(p.target_info.adapter_id, p.target_info.id))
            .collect()
    }
}

unsafe fn target_name(adapter_id: LUID, id: u32) -> Option<Target> {
    let mut name: DISPLAYCONFIG_TARGET_DEVICE_NAME = zeroed();
    name.header.kind = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
    name.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
    name.header.adapter_id = adapter_id;
    name.header.id = id;
    if DisplayConfigGetDeviceInfo(&mut name.header) != 0 {
        return None;
    }
    Some(Target {
        device_path: wide_to_string(&name.monitor_device_path).to_ascii_lowercase(),
        friendly_name: wide_to_string(&name.monitor_friendly_device_name),
        connection: connection_from_technology(name.output_technology),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_output_technologies() {
        assert_eq!(connection_from_technology(5), ConnectionKind::Hdmi);
        assert_eq!(connection_from_technology(10), ConnectionKind::DisplayPort);
        assert_eq!(connection_from_technology(11), ConnectionKind::DisplayPort);
        assert_eq!(connection_from_technology(19), ConnectionKind::UsbTunnel);
        assert_eq!(connection_from_technology(0x8000_0000), ConnectionKind::Other);
    }
}
