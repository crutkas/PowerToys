// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Sunrise/sunset calculation — NOAA algorithm.
//!
//! Pure math, no system calls. Timezone bias is injected for testability.

use std::f64::consts::PI;

fn deg2rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

fn rad2deg(rad: f64) -> f64 {
    rad * 180.0 / PI
}

/// Calculated sunrise and sunset times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SunTimes {
    pub sunrise_hour: i32,
    pub sunrise_minute: i32,
    pub sunset_hour: i32,
    pub sunset_minute: i32,
}

impl SunTimes {
    /// Sunrise as minutes since midnight.
    pub fn sunrise_minutes(&self) -> i32 {
        self.sunrise_hour * 60 + self.sunrise_minute
    }

    /// Sunset as minutes since midnight.
    pub fn sunset_minutes(&self) -> i32 {
        self.sunset_hour * 60 + self.sunset_minute
    }
}

/// Calculate sunrise and sunset times for a given date and location.
///
/// `timezone_bias_minutes` uses the Windows `TIME_ZONE_INFORMATION.Bias` convention:
/// positive means UTC is ahead of local time. For example, EST (UTC-5) has bias=300,
/// EDT (UTC-4) has bias=240.
///
/// Returns `None` if the sun doesn't rise or set at the given location/date
/// (polar regions).
pub fn calculate_sunrise_sunset(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: i32,
    day: i32,
    timezone_bias_minutes: i32,
) -> Option<SunTimes> {
    let zenith = 90.833;

    let n1 = (275.0 * month as f64 / 9.0).floor() as i32;
    let n2 = ((month as f64 + 9.0) / 12.0).floor() as i32;
    let n3 = (1.0 + ((year as f64 - 4.0 * (year as f64 / 4.0).floor() + 2.0) / 3.0).floor())
        as i32;
    let n = n1 - (n2 * n3) + day - 30;

    let calc_time = |sunrise: bool| -> Option<f64> {
        let lng_hour = longitude / 15.0;
        let t = if sunrise {
            n as f64 + ((6.0 - lng_hour) / 24.0)
        } else {
            n as f64 + ((18.0 - lng_hour) / 24.0)
        };

        let m = (0.9856 * t) - 3.289;
        let mut l = m + (1.916 * deg2rad(m).sin()) + (0.020 * (2.0 * deg2rad(m)).sin()) + 282.634;
        if l < 0.0 {
            l += 360.0;
        }
        if l > 360.0 {
            l -= 360.0;
        }

        let mut ra = rad2deg((0.91764 * deg2rad(l).tan()).atan());
        if ra < 0.0 {
            ra += 360.0;
        }
        if ra > 360.0 {
            ra -= 360.0;
        }

        let l_quadrant = (l / 90.0).floor() * 90.0;
        let ra_quadrant = (ra / 90.0).floor() * 90.0;
        ra = ra + (l_quadrant - ra_quadrant);
        ra /= 15.0;

        let sin_dec = 0.39782 * deg2rad(l).sin();
        let cos_dec = sin_dec.asin().cos();

        let cos_h = (deg2rad(zenith).cos() - (sin_dec * deg2rad(latitude).sin()))
            / (cos_dec * deg2rad(latitude).cos());

        if cos_h > 1.0 || cos_h < -1.0 {
            return None;
        }

        let h = if sunrise {
            360.0 - rad2deg(cos_h.acos())
        } else {
            rad2deg(cos_h.acos())
        };
        let h = h / 15.0;

        let big_t = h + ra - (0.06571 * t) - 6.622;
        let mut ut = big_t - lng_hour;
        while ut < 0.0 {
            ut += 24.0;
        }
        while ut >= 24.0 {
            ut -= 24.0;
        }

        Some(ut)
    };

    let rise_ut = calc_time(true)?;
    let set_ut = calc_time(false)?;

    let bias_hours = -(timezone_bias_minutes as f64 / 60.0);

    let to_local = |ut: f64| -> (i32, i32) {
        let mut local = ut + bias_hours;
        while local < 0.0 {
            local += 24.0;
        }
        while local >= 24.0 {
            local -= 24.0;
        }
        let hour = local as i32;
        let minute = ((local - hour as f64) * 60.0) as i32;
        (hour, minute)
    };

    let (rise_h, rise_m) = to_local(rise_ut);
    let (set_h, set_m) = to_local(set_ut);

    Some(SunTimes {
        sunrise_hour: rise_h,
        sunrise_minute: rise_m,
        sunset_hour: set_h,
        sunset_minute: set_m,
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_york_summer_solstice() {
        // June 21, 2024, New York (40.7128, -74.0060), EDT = UTC-4 (Windows bias = 240)
        let times = calculate_sunrise_sunset(40.7128, -74.0060, 2024, 6, 21, 240);
        assert!(times.is_some());
        let t = times.unwrap();
        // Sunrise around 5:25 AM, sunset around 8:30 PM
        assert!(t.sunrise_hour >= 5 && t.sunrise_hour <= 6, "sunrise_hour={}", t.sunrise_hour);
        assert!(t.sunset_hour >= 20 && t.sunset_hour <= 21, "sunset_hour={}", t.sunset_hour);
    }

    #[test]
    fn new_york_winter_solstice() {
        // Dec 21, 2024, New York, EST = UTC-5 (Windows bias = 300)
        let times = calculate_sunrise_sunset(40.7128, -74.0060, 2024, 12, 21, 300);
        assert!(times.is_some());
        let t = times.unwrap();
        // Sunrise around 7:16 AM, sunset around 4:31 PM
        assert!(t.sunrise_hour >= 7 && t.sunrise_hour <= 8, "sunrise_hour={}", t.sunrise_hour);
        assert!(t.sunset_hour >= 16 && t.sunset_hour <= 17, "sunset_hour={}", t.sunset_hour);
    }

    #[test]
    fn equator_equinox() {
        // Mar 20, 2024, East Africa (0, 30), EAT = UTC+2 (Windows bias = -120)
        let times = calculate_sunrise_sunset(0.0, 30.0, 2024, 3, 20, -120);
        assert!(times.is_some());
        let t = times.unwrap();
        // Near equinox at equator, sunrise ~6, sunset ~18
        assert!(t.sunrise_hour >= 5 && t.sunrise_hour <= 7, "sunrise_hour={}", t.sunrise_hour);
        assert!(t.sunset_hour >= 17 && t.sunset_hour <= 19, "sunset_hour={}", t.sunset_hour);
    }

    #[test]
    fn london_midsummer() {
        // June 21, 2024, London (51.5074, -0.1278), BST = UTC+1 (Windows bias = -60)
        let times = calculate_sunrise_sunset(51.5074, -0.1278, 2024, 6, 21, -60);
        assert!(times.is_some());
        let t = times.unwrap();
        // Sunrise ~4:43, sunset ~21:21
        assert!(t.sunrise_hour >= 4 && t.sunrise_hour <= 5, "sunrise_hour={}", t.sunrise_hour);
        assert!(t.sunset_hour >= 20 && t.sunset_hour <= 22, "sunset_hour={}", t.sunset_hour);
    }

    #[test]
    fn sunrise_before_sunset() {
        let times = calculate_sunrise_sunset(40.7128, -74.0060, 2024, 6, 21, 240).unwrap();
        assert!(times.sunrise_minutes() < times.sunset_minutes());
    }

    #[test]
    fn minutes_helpers() {
        let t = SunTimes {
            sunrise_hour: 6,
            sunrise_minute: 30,
            sunset_hour: 20,
            sunset_minute: 15,
        };
        assert_eq!(t.sunrise_minutes(), 390);
        assert_eq!(t.sunset_minutes(), 1215);
    }

    #[test]
    fn negative_longitude() {
        // San Francisco (37.7749, -122.4194), PDT = UTC-7 (Windows bias = 420)
        let times = calculate_sunrise_sunset(37.7749, -122.4194, 2024, 6, 21, 420);
        assert!(times.is_some());
        let t = times.unwrap();
        assert!(t.sunrise_hour >= 5 && t.sunrise_hour <= 6, "sunrise_hour={}", t.sunrise_hour);
        assert!(t.sunset_hour >= 20 && t.sunset_hour <= 21, "sunset_hour={}", t.sunset_hour);
    }
}
