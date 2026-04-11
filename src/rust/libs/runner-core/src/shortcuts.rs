use crate::hotkey_conflict::Hotkey;

/// Format a hotkey as a human-readable string, e.g. "Win + Ctrl + A".
pub fn shortcut_to_string(hotkey: &Hotkey) -> String {
    let mut parts: Vec<&str> = Vec::new();

    if hotkey.win {
        parts.push("Win");
    }
    if hotkey.ctrl {
        parts.push("Ctrl");
    }
    if hotkey.alt {
        parts.push("Alt");
    }
    if hotkey.shift {
        parts.push("Shift");
    }

    if hotkey.key != 0 {
        // Map common VK codes to readable names; fallback to the character
        // for printable ASCII keys.
        let key_name = match hotkey.key {
            0x08 => "Backspace".to_string(),
            0x09 => "Tab".to_string(),
            0x0D => "Enter".to_string(),
            0x1B => "Esc".to_string(),
            0x20 => "Space".to_string(),
            0x70..=0x87 => format!("F{}", hotkey.key - 0x70 + 1), // F1-F24
            b'A'..=b'Z' => String::from(hotkey.key as char),
            b'0'..=b'9' => String::from(hotkey.key as char),
            other => format!("0x{other:02X}"),
        };
        parts.push(Box::leak(key_name.into_boxed_str())); // static-lifetime trick OK for display
    }

    parts.join(" + ")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn hk(win: bool, ctrl: bool, shift: bool, alt: bool, key: u8) -> Hotkey {
        Hotkey {
            win,
            ctrl,
            shift,
            alt,
            key,
        }
    }

    #[test]
    fn win_only() {
        assert_eq!(shortcut_to_string(&hk(true, false, false, false, 0)), "Win");
    }

    #[test]
    fn ctrl_a() {
        assert_eq!(
            shortcut_to_string(&hk(false, true, false, false, b'A')),
            "Ctrl + A",
        );
    }

    #[test]
    fn win_ctrl_shift_alt_a() {
        assert_eq!(
            shortcut_to_string(&hk(true, true, true, true, b'A')),
            "Win + Ctrl + Alt + Shift + A",
        );
    }

    #[test]
    fn no_modifiers_key_only() {
        assert_eq!(shortcut_to_string(&hk(false, false, false, false, b'Z')), "Z");
    }

    #[test]
    fn f1_key() {
        assert_eq!(
            shortcut_to_string(&hk(false, false, false, false, 0x70)),
            "F1",
        );
    }

    #[test]
    fn shift_f12() {
        assert_eq!(
            shortcut_to_string(&hk(false, false, true, false, 0x7B)),
            "Shift + F12",
        );
    }

    #[test]
    fn alt_tab() {
        assert_eq!(
            shortcut_to_string(&hk(false, false, false, true, 0x09)),
            "Alt + Tab",
        );
    }

    #[test]
    fn empty_hotkey() {
        assert_eq!(shortcut_to_string(&hk(false, false, false, false, 0)), "");
    }

    #[test]
    fn win_space() {
        assert_eq!(
            shortcut_to_string(&hk(true, false, false, false, 0x20)),
            "Win + Space",
        );
    }

    #[test]
    fn numeric_key() {
        assert_eq!(
            shortcut_to_string(&hk(false, true, false, false, b'5')),
            "Ctrl + 5",
        );
    }
}
