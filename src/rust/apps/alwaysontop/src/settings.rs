//! AlwaysOnTop settings — loads from the PowerToys settings JSON file.

#[derive(Debug, Clone)]
pub struct Settings {
    pub frame_enabled: bool,
    pub frame_thickness: i32,
    pub frame_color: u32,     // COLORREF (0x00BBGGRR)
    pub frame_opacity: i32,   // 0-100
    pub frame_accent_color: bool,
    pub round_corners_enabled: bool,
    pub sound_enabled: bool,
    pub block_in_game_mode: bool,
    pub show_in_system_menu: bool,
    pub excluded_apps: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            frame_enabled: true,
            frame_thickness: 15,
            frame_color: 0x00EFAD00, // RGB(0, 173, 239) in COLORREF = BGR
            frame_opacity: 100,
            frame_accent_color: true,
            round_corners_enabled: true,
            sound_enabled: true,
            block_in_game_mode: true,
            show_in_system_menu: false,
            excluded_apps: Vec::new(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Self::settings_path();
        let json = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };

        let raw: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };

        let props = match raw.get("properties") {
            Some(p) => p,
            None => return Self::default(),
        };

        let mut s = Self::default();

        if let Some(v) = get_bool(props, "frame-enabled") { s.frame_enabled = v; }
        if let Some(v) = get_int(props, "frame-thickness") { s.frame_thickness = v; }
        if let Some(v) = get_int(props, "frame-opacity") { s.frame_opacity = v; }
        if let Some(v) = get_bool(props, "frame-accent-color") { s.frame_accent_color = v; }
        if let Some(v) = get_bool(props, "round-corners-enabled") { s.round_corners_enabled = v; }
        if let Some(v) = get_bool(props, "sound-enabled") { s.sound_enabled = v; }
        if let Some(v) = get_bool(props, "do-not-activate-on-game-mode") { s.block_in_game_mode = v; }
        if let Some(v) = get_bool(props, "show-in-system-menu") { s.show_in_system_menu = v; }

        // Parse frame color from hex string like "#00ADEF"
        if let Some(color_str) = get_str(props, "frame-color") {
            if let Some(hex) = color_str.strip_prefix('#') {
                if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                    // Convert RGB to COLORREF (BGR)
                    let r = (rgb >> 16) & 0xFF;
                    let g = (rgb >> 8) & 0xFF;
                    let b = rgb & 0xFF;
                    s.frame_color = r | (g << 8) | (b << 16);
                }
            }
        }

        // Parse excluded apps (newline-separated string)
        if let Some(apps_str) = get_str(props, "excluded-apps") {
            s.excluded_apps = apps_str.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
        }

        s
    }

    /// Return (R, G, B) tuple from the COLORREF value.
    pub fn frame_color_rgb(&self) -> (u8, u8, u8) {
        let r = (self.frame_color & 0xFF) as u8;
        let g = ((self.frame_color >> 8) & 0xFF) as u8;
        let b = ((self.frame_color >> 16) & 0xFF) as u8;
        (r, g, b)
    }

    fn settings_path() -> String {
        let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
        format!("{}\\Microsoft\\PowerToys\\AlwaysOnTop\\settings.json", local)
    }
}

fn get_bool(props: &serde_json::Value, key: &str) -> Option<bool> {
    props.get(key)?.get("value")?.as_bool()
}

fn get_int(props: &serde_json::Value, key: &str) -> Option<i32> {
    props.get(key)?.get("value")?.as_i64().map(|v| v as i32)
}

fn get_str<'a>(props: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    props.get(key)?.get("value")?.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert!(s.frame_enabled);
        assert_eq!(s.frame_thickness, 15);
        assert_eq!(s.frame_opacity, 100);
        assert!(s.sound_enabled);
    }

    #[test]
    fn test_frame_color_rgb() {
        let mut s = Settings::default();
        s.frame_color = 0x00FF8000; // R=0, G=128, B=255 in COLORREF
        let (r, g, b) = s.frame_color_rgb();
        assert_eq!(r, 0);
        assert_eq!(g, 128);
        assert_eq!(b, 255);
    }
}
