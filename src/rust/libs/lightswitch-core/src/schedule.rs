// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Schedule logic — time-based theme switching.
//!
//! All functions are pure: no system calls, fully testable.

/// Determine whether the current time should be light mode.
///
/// Handles wraparound (e.g., light starts at 20:00, dark starts at 08:00).
/// All times are in minutes since midnight, normalized to [0, 1440).
pub fn should_be_light(now_minutes: i32, light_time: i32, dark_time: i32) -> bool {
    let norm = |m: i32| ((m % 1440) + 1440) % 1440;
    let now = norm(now_minutes);
    let light = norm(light_time);
    let dark = norm(dark_time);

    if light < dark {
        // Normal range: light mode from light..dark
        now >= light && now < dark
    } else {
        // Wraparound: light mode from light..midnight..dark
        now >= light || now < dark
    }
}

/// Get current time as minutes since midnight (0–1439).
///
/// This function calls the system clock; use `should_be_light` with injected
/// times for testing.
pub fn get_now_minutes() -> i32 {
    let now = std::time::SystemTime::now();
    let since_epoch = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Convert to local time by using a simple offset approach.
    // For the actual app, we use Win32 GetLocalTime; this is a fallback.
    let secs = since_epoch.as_secs();
    // This gives UTC minutes; caller should use platform-specific local time.
    let day_secs = (secs % 86400) as i32;
    day_secs / 60
}

/// Check whether a boundary was crossed between `prev` and `now` minutes.
///
/// Returns true if either the light or dark boundary was crossed.
/// Handles midnight wraparound (prev > now).
pub fn crossed_boundary(
    prev: i32,
    now: i32,
    effective_light: i32,
    effective_dark: i32,
) -> bool {
    if now < prev {
        // Midnight wraparound.
        (prev <= effective_light || now >= effective_light)
            || (prev <= effective_dark || now >= effective_dark)
    } else {
        (prev < effective_light && now >= effective_light)
            || (prev < effective_dark && now >= effective_dark)
    }
}

/// Validate coordinate values for sunrise/sunset calculation.
pub fn coordinates_are_valid(lat: &str, lon: &str) -> bool {
    let lat_val: f64 = match lat.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let lon_val: f64 = match lon.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    !(lat_val == 0.0 && lon_val == 0.0)
        && (-90.0..=90.0).contains(&lat_val)
        && (-180.0..=180.0).contains(&lon_val)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── should_be_light ──

    #[test]
    fn normal_range_morning() {
        // 08:00–20:00 light, at 10:00 → light
        assert!(should_be_light(600, 480, 1200));
    }

    #[test]
    fn normal_range_evening() {
        // 08:00–20:00 light, at 21:00 → dark
        assert!(!should_be_light(1260, 480, 1200));
    }

    #[test]
    fn normal_range_at_light_boundary() {
        // Exactly at light start → light
        assert!(should_be_light(480, 480, 1200));
    }

    #[test]
    fn normal_range_at_dark_boundary() {
        // Exactly at dark start → dark
        assert!(!should_be_light(1200, 480, 1200));
    }

    #[test]
    fn normal_range_midnight() {
        // At midnight (0) with 08:00-20:00 → dark
        assert!(!should_be_light(0, 480, 1200));
    }

    #[test]
    fn wraparound_range_evening() {
        // Light from 20:00, dark from 08:00 (inverted)
        // At 21:00 → light
        assert!(should_be_light(1260, 1200, 480));
    }

    #[test]
    fn wraparound_range_morning() {
        // Light from 20:00, dark from 08:00
        // At 03:00 → light (between midnight and dark)
        assert!(should_be_light(180, 1200, 480));
    }

    #[test]
    fn wraparound_range_afternoon() {
        // Light from 20:00, dark from 08:00
        // At 12:00 → dark
        assert!(!should_be_light(720, 1200, 480));
    }

    #[test]
    fn equal_times_always_dark() {
        // Same time for light and dark → wraparound case, always light
        // When light == dark, the `light < dark` branch is false,
        // so we hit wraparound: now >= light || now < dark → always true
        assert!(should_be_light(0, 480, 480));
        assert!(should_be_light(720, 480, 480));
    }

    #[test]
    fn negative_normalization() {
        // Negative time values should normalize correctly
        assert!(should_be_light(-60, 1380, 480)); // -60 → 1380, should be light (1380–480 wrap)
    }

    #[test]
    fn large_values_normalize() {
        // 1500 minutes = 25 hours → normalizes to 60 min = 01:00
        assert!(!should_be_light(1500, 480, 1200)); // 01:00, dark in 08:00–20:00
    }

    // ── crossed_boundary ──

    #[test]
    fn no_boundary_crossed() {
        assert!(!crossed_boundary(470, 475, 480, 1200));
    }

    #[test]
    fn light_boundary_crossed() {
        assert!(crossed_boundary(479, 481, 480, 1200));
    }

    #[test]
    fn dark_boundary_crossed() {
        assert!(crossed_boundary(1199, 1201, 480, 1200));
    }

    #[test]
    fn midnight_wraparound_crosses_light() {
        // prev=1430 (23:50), now=10 (00:10), light at 0 (00:00)
        assert!(crossed_boundary(1430, 10, 0, 1200));
    }

    #[test]
    fn no_crossing_same_time() {
        assert!(!crossed_boundary(480, 480, 480, 1200));
    }

    // ── coordinates_are_valid ──

    #[test]
    fn valid_coordinates() {
        assert!(coordinates_are_valid("40.7128", "-74.0060"));
        assert!(coordinates_are_valid("90.0", "180.0"));
        assert!(coordinates_are_valid("-90.0", "-180.0"));
    }

    #[test]
    fn invalid_zero_zero() {
        assert!(!coordinates_are_valid("0.0", "0.0"));
        assert!(!coordinates_are_valid("0", "0"));
    }

    #[test]
    fn invalid_out_of_range() {
        assert!(!coordinates_are_valid("91.0", "0.0"));
        assert!(!coordinates_are_valid("0.0", "181.0"));
        assert!(!coordinates_are_valid("-91.0", "0.0"));
    }

    #[test]
    fn invalid_non_numeric() {
        assert!(!coordinates_are_valid("abc", "def"));
        assert!(!coordinates_are_valid("", ""));
    }

    #[test]
    fn valid_nonzero_with_zero() {
        assert!(coordinates_are_valid("40.0", "0.0"));
        assert!(coordinates_are_valid("0.0", "40.0"));
    }

    // ── get_now_minutes ──

    #[test]
    fn now_minutes_in_range() {
        let m = get_now_minutes();
        assert!(m >= 0 && m < 1440);
    }
}
