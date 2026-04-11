//! Low-level keyboard hook for detecting held letter + trigger key.

use crate::accents;
use crate::popup;
use crate::input;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

static HOOK: std::sync::Mutex<isize> = std::sync::Mutex::new(0);
static LETTER_HELD: AtomicU32 = AtomicU32::new(0);    // VK code of held letter (0 = none)
static POPUP_VISIBLE: AtomicBool = AtomicBool::new(false);
static SELECTION_INDEX: AtomicU32 = AtomicU32::new(0);
static mut KEY_DOWN_TIME: Option<Instant> = None;

const INPUT_DELAY_MS: u128 = 200; // ms before showing popup (avoid false triggers)

pub fn install_hook() {
    unsafe {
        let h = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(ll_keyboard_proc),
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null()),
            0,
        );
        if !h.is_null() {
            *HOOK.lock().unwrap() = h as isize;
        }
    }
}

pub fn uninstall_hook() {
    let h = *HOOK.lock().unwrap();
    if h != 0 {
        unsafe { UnhookWindowsHookEx(h as *mut _); }
    }
}

unsafe extern "system" fn ll_keyboard_proc(n_code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if n_code < 0 {
            return CallNextHookEx(std::ptr::null_mut(), n_code, wparam, lparam);
        }

        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode;

        match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                handle_key_down(vk);
            }
            WM_KEYUP | WM_SYSKEYUP => {
                handle_key_up(vk);
            }
            _ => {}
        }

        // If popup is visible and we're handling a trigger key, swallow it
        if POPUP_VISIBLE.load(Ordering::SeqCst) && is_trigger_key(vk) && wparam as u32 == WM_KEYDOWN {
            return 1; // Swallow the key
        }

        CallNextHookEx(std::ptr::null_mut(), n_code, wparam, lparam)
    }
}

fn handle_key_down(vk: u32) {
    let letter = LETTER_HELD.load(Ordering::SeqCst);

    if is_letter_key(vk) {
        if letter == 0 {
            // Start tracking this letter
            LETTER_HELD.store(vk, Ordering::SeqCst);
            unsafe { KEY_DOWN_TIME = Some(Instant::now()); }
        }
        return;
    }

    if letter == 0 { return; }

    // A letter is held — check for trigger keys
    if !is_trigger_key(vk) {
        // Non-trigger key pressed while holding letter — cancel
        cancel();
        return;
    }

    // Check if held long enough
    let elapsed = unsafe { KEY_DOWN_TIME.map(|t| t.elapsed().as_millis()).unwrap_or(0) };
    if elapsed < INPUT_DELAY_MS {
        return;
    }

    if !POPUP_VISIBLE.load(Ordering::SeqCst) {
        // First trigger — show popup
        let chars = accents::get_accents(letter);
        if chars.is_empty() { return; }

        SELECTION_INDEX.store(0, Ordering::SeqCst);
        popup::show(&chars, 0);
        POPUP_VISIBLE.store(true, Ordering::SeqCst);
    } else {
        // Navigate selection
        let chars = accents::get_accents(letter);
        let count = chars.len() as u32;
        if count == 0 { return; }

        let mut idx = SELECTION_INDEX.load(Ordering::SeqCst);
        match vk {
            x if x == VK_SPACE as u32 || x == VK_RIGHT as u32 => { idx = (idx + 1) % count; }
            x if x == VK_LEFT as u32 => { idx = if idx == 0 { count - 1 } else { idx - 1 }; }
            _ => {}
        }
        SELECTION_INDEX.store(idx, Ordering::SeqCst);
        popup::show(&chars, idx as usize);
    }
}

fn handle_key_up(vk: u32) {
    let letter = LETTER_HELD.load(Ordering::SeqCst);
    if vk != letter { return; }

    // Letter released
    if POPUP_VISIBLE.load(Ordering::SeqCst) {
        let chars = accents::get_accents(letter);
        let idx = SELECTION_INDEX.load(Ordering::SeqCst) as usize;
        if idx < chars.len() {
            // Delete the original letter, then type the accent
            input::backspace_and_type(chars[idx]);
        }
        popup::hide();
        POPUP_VISIBLE.store(false, Ordering::SeqCst);
    }

    LETTER_HELD.store(0, Ordering::SeqCst);
    unsafe { KEY_DOWN_TIME = None; }
}

fn cancel() {
    if POPUP_VISIBLE.load(Ordering::SeqCst) {
        popup::hide();
        POPUP_VISIBLE.store(false, Ordering::SeqCst);
    }
    LETTER_HELD.store(0, Ordering::SeqCst);
    unsafe { KEY_DOWN_TIME = None; }
}

fn is_letter_key(vk: u32) -> bool {
    (0x30..=0x39).contains(&vk) || // 0-9
    (0x41..=0x5A).contains(&vk)    // A-Z
}

fn is_trigger_key(vk: u32) -> bool {
    vk == VK_SPACE as u32 || vk == VK_LEFT as u32 || vk == VK_RIGHT as u32
}
