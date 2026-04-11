//! Input simulation — backspace + type accent character via SendInput.

use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

/// Delete the original letter with backspace, then type the accent character.
pub fn backspace_and_type(ch: char) {
    // Send backspace to delete the original letter
    send_key(VK_BACK as u16, true);
    send_key(VK_BACK as u16, false);

    // Type the accent character using Unicode input
    send_unicode_char(ch);
}

fn send_key(vk: u16, down: bool) {
    let mut input: INPUT = unsafe { std::mem::zeroed() };
    input.r#type = INPUT_KEYBOARD;
    let ki = unsafe { &mut input.Anonymous.ki };
    ki.wVk = vk;
    ki.dwFlags = if down { 0 } else { KEYEVENTF_KEYUP };
    unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32); }
}

fn send_unicode_char(ch: char) {
    let mut buf = [0u16; 2];
    let encoded = ch.encode_utf16(&mut buf);

    for &code_unit in encoded.iter() {
        // Key down
        let mut input_down: INPUT = unsafe { std::mem::zeroed() };
        input_down.r#type = INPUT_KEYBOARD;
        let ki_down = unsafe { &mut input_down.Anonymous.ki };
        ki_down.wScan = code_unit;
        ki_down.dwFlags = KEYEVENTF_UNICODE;

        // Key up
        let mut input_up: INPUT = unsafe { std::mem::zeroed() };
        input_up.r#type = INPUT_KEYBOARD;
        let ki_up = unsafe { &mut input_up.Anonymous.ki };
        ki_up.wScan = code_unit;
        ki_up.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;

        let inputs = [input_down, input_up];
        unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32); }
    }
}
