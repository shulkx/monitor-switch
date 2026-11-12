//! Keyboard helpers: which modifiers are held, hiding a released Alt/Win from the focused
//! program, and the hook that records a hotkey even when another program owns it.

use std::mem::{size_of, zeroed};
use std::ptr::null;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc, Mutex};
use std::thread;

use super::ffi;

/// Unassigned virtual key. Pressing it "uses" a held Alt or Win without meaning anything else.
const MASK_KEY: u16 = 0xE8;

fn is_down(vk: i32) -> bool {
    (unsafe { ffi::GetAsyncKeyState(vk) } as u16) & 0x8000 != 0
}

/// True while Ctrl, Alt, Shift or Win is physically held.
pub fn modifiers_down() -> bool {
    [ffi::VK_CONTROL, ffi::VK_MENU, ffi::VK_SHIFT, ffi::VK_LWIN, ffi::VK_RWIN]
        .into_iter()
        .any(is_down)
}

/// A hotkey's last key never reaches the focused program, so it sees Alt (or Win) pressed and
/// released on its own: that opens its menu bar (or the Start menu) and typing goes nowhere.
/// Tapping an unused key while the modifier is still held prevents that.
pub fn mask_modifier_release() {
    let input = |flags| ffi::INPUT {
        kind: ffi::INPUT_KEYBOARD,
        ki: ffi::KEYBDINPUT { vk: MASK_KEY, scan: 0, flags, time: 0, extra_info: 0 },
        _pad: [0; 8],
    };
    let inputs = [input(0), input(ffi::KEYEVENTF_KEYUP)];
    unsafe { ffi::SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<ffi::INPUT>() as i32) };
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

fn held() -> Held {
    Held {
        ctrl: is_down(ffi::VK_CONTROL),
        alt: is_down(ffi::VK_MENU),
        shift: is_down(ffi::VK_SHIFT),
        win: is_down(ffi::VK_LWIN) || is_down(ffi::VK_RWIN),
    }
}

/// The accelerator for a key pressed with `held` modifiers, in the same form the settings page
/// builds from KeyboardEvent.code ("Ctrl+Alt+M", "Ctrl+Shift+F5", "Alt+Super+Numpad1").
/// None for keys the page handles itself: anything without Ctrl/Alt/Win, Esc (cancel),
/// Backspace/Delete (clear), and keys global hotkeys can't use.
pub fn accelerator_for(vk: u32, held: Held) -> Option<String> {
    if !(held.ctrl || held.alt || held.win) {
        return None;
    }
    let key = key_name(vk)?;
    let mut parts = Vec::new();
    if held.ctrl {
        parts.push("Ctrl".to_string());
    }
    if held.alt {
        parts.push("Alt".to_string());
    }
    if held.shift {
        parts.push("Shift".to_string());
    }
    if held.win {
        parts.push("Super".to_string());
    }
    parts.push(key);
    Some(parts.join("+"))
}

fn key_name(vk: u32) -> Option<String> {
    let name = match vk {
        0x41..=0x5A | 0x30..=0x39 => return char::from_u32(vk).map(String::from),
        0x70..=0x87 => return Some(format!("F{}", vk - 0x6F)),
        0x60..=0x69 => return Some(format!("Numpad{}", vk - 0x60)),
        0x20 => "Space",
        0x0D => "Enter",
        0x09 => "Tab",
        0x21 => "PageUp",
        0x22 => "PageDown",
        0x23 => "End",
        0x24 => "Home",
        0x25 => "ArrowLeft",
        0x26 => "ArrowUp",
        0x27 => "ArrowRight",
        0x28 => "ArrowDown",
        0x2D => "Insert",
        0xBA => "Semicolon",
        0xBB => "Equal",
        0xBC => "Comma",
        0xBD => "Minus",
        0xBE => "Period",
        0xBF => "Slash",
        0xC0 => "Backquote",
        0xDB => "BracketLeft",
        0xDC => "Backslash",
        0xDD => "BracketRight",
        0xDE => "Quote",
        _ => return None,
    };
    Some(name.to_string())
}

type Sink = Box<dyn FnOnce(String) + Send>;

/// Where the next recorded combination goes. Taken (emptied) by the first one.
static SINK: Mutex<Option<Sink>> = Mutex::new(None);
/// The hook thread, so `stop_recording` can end its message loop. 0 when not running.
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);

/// Catch the next Ctrl/Alt/Win combination pressed while one of our windows is in front,
/// before any program's global hotkey sees it, and hand it to `on_combo` (once).
/// The keystroke is swallowed, so recording a combination another program owns doesn't
/// trigger that program. Returns false if the hook couldn't be installed.
pub fn start_recording(on_combo: impl FnOnce(String) + Send + 'static) -> bool {
    stop_recording();
    *SINK.lock().unwrap() = Some(Box::new(on_combo));
    let (ready, installed) = mpsc::channel();
    thread::spawn(move || unsafe {
        let mut msg: ffi::MSG = zeroed();
        // Create this thread's message queue before anyone posts WM_QUIT to it.
        ffi::PeekMessageW(&mut msg, 0, 0, 0, ffi::PM_NOREMOVE);
        let hook = ffi::SetWindowsHookExW(ffi::WH_KEYBOARD_LL, Some(hook_proc), ffi::GetModuleHandleW(null()), 0);
        if hook == 0 {
            let _ = ready.send(false);
            return;
        }
        HOOK_THREAD.store(ffi::GetCurrentThreadId(), Ordering::SeqCst);
        let _ = ready.send(true);
        // Low-level hooks run inside this loop.
        while ffi::GetMessageW(&mut msg, 0, 0, 0) > 0 {}
        ffi::UnhookWindowsHookEx(hook);
    });
    let ok = installed.recv().unwrap_or(false);
    if !ok {
        SINK.lock().unwrap().take();
    }
    ok
}

pub fn stop_recording() {
    SINK.lock().unwrap().take();
    let thread_id = HOOK_THREAD.swap(0, Ordering::SeqCst);
    if thread_id != 0 {
        unsafe { ffi::PostThreadMessageW(thread_id, ffi::WM_QUIT, 0, 0) };
    }
}

/// True when the window in front belongs to this process (the settings window while recording).
fn our_window_in_front() -> bool {
    let mut pid = 0u32;
    unsafe { ffi::GetWindowThreadProcessId(ffi::GetForegroundWindow(), &mut pid) };
    pid == unsafe { ffi::GetCurrentProcessId() }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    let key_down = wparam == ffi::WM_KEYDOWN as usize || wparam == ffi::WM_SYSKEYDOWN as usize;
    if code == ffi::HC_ACTION && key_down {
        let info = &*(lparam as *const ffi::KBDLLHOOKSTRUCT);
        if info.flags & ffi::LLKHF_INJECTED == 0 && our_window_in_front() {
            if let Some(accelerator) = accelerator_for(info.vk_code, held()) {
                let sink = SINK.lock().unwrap().take();
                if let Some(sink) = sink {
                    // The settings window would otherwise see a lone Alt and enter menu mode.
                    mask_modifier_release();
                    sink(accelerator);
                    return 1;
                }
            }
        }
    }
    ffi::CallNextHookEx(0, code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTRL_ALT: Held = Held { ctrl: true, alt: true, shift: false, win: false };

    #[test]
    fn names_combinations_like_the_settings_page() {
        assert_eq!(accelerator_for(0x4D, CTRL_ALT).as_deref(), Some("Ctrl+Alt+M"));
        assert_eq!(accelerator_for(0x31, CTRL_ALT).as_deref(), Some("Ctrl+Alt+1"));
        let all = Held { ctrl: true, alt: true, shift: true, win: true };
        assert_eq!(accelerator_for(0x74, all).as_deref(), Some("Ctrl+Alt+Shift+Super+F5"));
        assert_eq!(accelerator_for(0x61, CTRL_ALT).as_deref(), Some("Ctrl+Alt+Numpad1"));
        assert_eq!(accelerator_for(0xC0, CTRL_ALT).as_deref(), Some("Ctrl+Alt+Backquote"));
    }

    #[test]
    fn leaves_plain_and_editing_keys_to_the_page() {
        assert_eq!(accelerator_for(0x4D, Held::default()), None);
        assert_eq!(accelerator_for(0x4D, Held { shift: true, ..Held::default() }), None);
        assert_eq!(accelerator_for(0x1B, CTRL_ALT), None); // Esc cancels
        assert_eq!(accelerator_for(0x08, CTRL_ALT), None); // Backspace clears
        assert_eq!(accelerator_for(0x2E, CTRL_ALT), None); // Delete clears
        assert_eq!(accelerator_for(0x11, CTRL_ALT), None); // a modifier on its own
    }

    #[test]
    fn recorded_names_are_valid_accelerators() {
        for vk in [0x4D, 0x31, 0x74, 0x61, 0x20, 0x25, 0xBA, 0xC0, 0xDB, 0xDE] {
            let accelerator = accelerator_for(vk, CTRL_ALT).unwrap();
            assert!(crate::hotkeys::shortcut_id(&accelerator).is_ok(), "{accelerator}");
        }
    }
}
