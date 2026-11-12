//! Raw Win32 declarations. Handles are pointer-sized integers (isize), which keeps them Send.
#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MONITORINFOEXW {
    pub cb_size: u32,
    pub rc_monitor: RECT,
    pub rc_work: RECT,
    pub flags: u32,
    pub sz_device: [u16; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAY_DEVICEW {
    pub cb: u32,
    pub device_name: [u16; 32],
    pub device_string: [u16; 128],
    pub state_flags: u32,
    pub device_id: [u16; 128],
    pub device_key: [u16; 128],
}

/// Declared with pack(1) in the SDK; on 32- and 64-bit Windows the layout is the same as repr(C).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PHYSICAL_MONITOR {
    pub handle: isize,
    pub description: [u16; 128],
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct LUID {
    pub low: u32,
    pub high: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_RATIONAL {
    pub numerator: u32,
    pub denominator: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_PATH_SOURCE_INFO {
    pub adapter_id: LUID,
    pub id: u32,
    pub mode_info_idx: u32,
    pub status_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_PATH_TARGET_INFO {
    pub adapter_id: LUID,
    pub id: u32,
    pub mode_info_idx: u32,
    pub output_technology: u32,
    pub rotation: u32,
    pub scaling: u32,
    pub refresh_rate: DISPLAYCONFIG_RATIONAL,
    pub scan_line_ordering: u32,
    pub target_available: i32,
    pub status_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_PATH_INFO {
    pub source_info: DISPLAYCONFIG_PATH_SOURCE_INFO,
    pub target_info: DISPLAYCONFIG_PATH_TARGET_INFO,
    pub flags: u32,
}

/// The union payload is never read; only its size (48 bytes, 8-byte aligned) matters.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_MODE_INFO {
    pub info_type: u32,
    pub id: u32,
    pub adapter_id: LUID,
    pub payload: [u64; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_DEVICE_INFO_HEADER {
    pub kind: u32,
    pub size: u32,
    pub adapter_id: LUID,
    pub id: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_TARGET_DEVICE_NAME {
    pub header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    pub flags: u32,
    pub output_technology: u32,
    pub edid_manufacture_id: u16,
    pub edid_product_code_id: u16,
    pub connector_instance: u32,
    pub monitor_friendly_device_name: [u16; 64],
    pub monitor_device_path: [u16; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SYSTEM_POWER_STATUS {
    pub ac_line_status: u8,
    pub battery_flag: u8,
    pub battery_life_percent: u8,
    pub system_status_flag: u8,
    pub battery_life_time: u32,
    pub battery_full_life_time: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MSG {
    pub hwnd: isize,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub pt: POINT,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KBDLLHOOKSTRUCT {
    pub vk_code: u32,
    pub scan_code: u32,
    pub flags: u32,
    pub time: u32,
    pub extra_info: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KEYBDINPUT {
    pub vk: u16,
    pub scan: u16,
    pub flags: u32,
    pub time: u32,
    pub extra_info: usize,
}

/// INPUT with the keyboard member of the union; the padding covers MOUSEINPUT, the largest member.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct INPUT {
    pub kind: u32,
    pub ki: KEYBDINPUT,
    pub _pad: [u8; 8],
}

pub type HookProc = unsafe extern "system" fn(i32, usize, isize) -> isize;

pub const WH_KEYBOARD_LL: i32 = 13;
pub const HC_ACTION: i32 = 0;
pub const WM_QUIT: u32 = 0x0012;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const LLKHF_INJECTED: u32 = 0x10;
pub const PM_NOREMOVE: u32 = 0;
pub const INPUT_KEYBOARD: u32 = 1;
pub const KEYEVENTF_KEYUP: u32 = 0x2;
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_LWIN: i32 = 0x5B;
pub const VK_RWIN: i32 = 0x5C;
pub const SW_HIDE: i32 = 0;
pub const HWND_TOPMOST: isize = -1;
pub const SWP_NOSIZE: u32 = 0x1;
pub const SWP_NOMOVE: u32 = 0x2;
pub const SWP_NOACTIVATE: u32 = 0x10;
pub const SWP_SHOWWINDOW: u32 = 0x40;
pub const SWP_ASYNCWINDOWPOS: u32 = 0x4000;

pub const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x1;
pub const DISPLAY_DEVICE_ACTIVE: u32 = 0x1;
pub const QDC_ONLY_ACTIVE_PATHS: u32 = 0x2;
pub const DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME: u32 = 2;
pub const ERROR_INSUFFICIENT_BUFFER: i32 = 122;
/// HKEY_LOCAL_MACHINE is the sign-extended 32-bit value 0x80000002.
pub const HKEY_LOCAL_MACHINE: isize = 0x8000_0002u32 as i32 as isize;
pub const RRF_RT_REG_BINARY: u32 = 0x8;
pub const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
pub const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

pub type MonitorEnumProc = unsafe extern "system" fn(isize, isize, *mut RECT, isize) -> i32;

#[link(name = "user32")]
extern "system" {
    pub fn EnumDisplayMonitors(hdc: isize, clip: *const RECT, callback: Option<MonitorEnumProc>, data: isize) -> i32;
    pub fn GetMonitorInfoW(monitor: isize, info: *mut MONITORINFOEXW) -> i32;
    pub fn EnumDisplayDevicesW(device: *const u16, index: u32, info: *mut DISPLAY_DEVICEW, flags: u32) -> i32;
    pub fn GetDisplayConfigBufferSizes(flags: u32, path_count: *mut u32, mode_count: *mut u32) -> i32;
    pub fn QueryDisplayConfig(
        flags: u32,
        path_count: *mut u32,
        paths: *mut DISPLAYCONFIG_PATH_INFO,
        mode_count: *mut u32,
        modes: *mut DISPLAYCONFIG_MODE_INFO,
        topology: *mut u32,
    ) -> i32;
    pub fn DisplayConfigGetDeviceInfo(header: *mut DISPLAYCONFIG_DEVICE_INFO_HEADER) -> i32;
    pub fn SetWindowsHookExW(id: i32, proc_: Option<HookProc>, module: isize, thread_id: u32) -> isize;
    pub fn CallNextHookEx(hook: isize, code: i32, wparam: usize, lparam: isize) -> isize;
    pub fn UnhookWindowsHookEx(hook: isize) -> i32;
    pub fn GetMessageW(msg: *mut MSG, hwnd: isize, min: u32, max: u32) -> i32;
    pub fn PeekMessageW(msg: *mut MSG, hwnd: isize, min: u32, max: u32, remove: u32) -> i32;
    pub fn PostThreadMessageW(thread_id: u32, msg: u32, wparam: usize, lparam: isize) -> i32;
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn SendInput(count: u32, inputs: *const INPUT, size: i32) -> u32;
    pub fn GetForegroundWindow() -> isize;
    pub fn GetWindowThreadProcessId(hwnd: isize, process_id: *mut u32) -> u32;
    pub fn ShowWindowAsync(hwnd: isize, cmd: i32) -> i32;
    pub fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
}

#[link(name = "dxva2")]
extern "system" {
    pub fn GetNumberOfPhysicalMonitorsFromHMONITOR(monitor: isize, count: *mut u32) -> i32;
    pub fn GetPhysicalMonitorsFromHMONITOR(monitor: isize, count: u32, monitors: *mut PHYSICAL_MONITOR) -> i32;
    pub fn DestroyPhysicalMonitor(handle: isize) -> i32;
    pub fn GetVCPFeatureAndVCPFeatureReply(handle: isize, code: u8, code_type: *mut u32, current: *mut u32, maximum: *mut u32) -> i32;
    pub fn SetVCPFeature(handle: isize, code: u8, value: u32) -> i32;
    pub fn GetCapabilitiesStringLength(handle: isize, length: *mut u32) -> i32;
    pub fn CapabilitiesRequestAndCapabilitiesReply(handle: isize, buffer: *mut u8, length: u32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetSystemPowerStatus(status: *mut SYSTEM_POWER_STATUS) -> i32;
    pub fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    pub fn GetCurrentThreadId() -> u32;
    pub fn GetCurrentProcessId() -> u32;
    pub fn GetModuleHandleW(name: *const u16) -> isize;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegGetValueW(
        key: isize,
        sub_key: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut u8,
        size: *mut u32,
    ) -> i32;
}

pub fn wide_to_string(wide: &[u16]) -> String {
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..end])
}

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    /// Sizes from the Windows SDK headers. A mismatch means Windows would write past our buffers.
    #[test]
    fn struct_sizes_match_the_sdk() {
        assert_eq!(size_of::<DISPLAYCONFIG_PATH_INFO>(), 72);
        assert_eq!(size_of::<DISPLAYCONFIG_MODE_INFO>(), 64);
        assert_eq!(size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>(), 420);
        assert_eq!(size_of::<DISPLAY_DEVICEW>(), 840);
        assert_eq!(size_of::<MONITORINFOEXW>(), 104);
        assert_eq!(size_of::<SYSTEM_POWER_STATUS>(), 12);
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(size_of::<MSG>(), 48);
            assert_eq!(size_of::<KBDLLHOOKSTRUCT>(), 24);
            assert_eq!(size_of::<INPUT>(), 40);
        }
    }

    #[test]
    fn converts_wide_strings() {
        assert_eq!(wide_to_string(&[0x44, 0x50, 0, 0x58]), "DP");
        assert_eq!(to_wide("A"), vec![0x41, 0]);
    }
}
