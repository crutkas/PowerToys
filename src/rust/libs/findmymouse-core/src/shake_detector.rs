// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Mouse shake detection algorithm.
//!
//! Mirrors the C++ `DetectShake` / `OnSonarMouseInput` logic. All timing
//! is injected via explicit timestamps — no Win32 calls.
//!
//! Algorithm:
//! 1. Track mouse movement history with timestamps.
//! 2. Merge consecutive same-direction movements to keep history small.
//! 3. On direction change, check if total_distance / bounding_diagonal > shake_factor.
//! 4. Prune movements older than shake_interval_ms.

/// A single recorded mouse movement segment.
#[derive(Debug, Clone)]
struct Movement {
    dx: i64,
    dy: i64,
    tick: u64,
}

/// Sign function matching the C++ `GetSign`.
fn sign(n: i64) -> i8 {
    if n > 0 {
        1
    } else if n < 0 {
        -1
    } else {
        0
    }
}

/// Mouse shake detector.
pub struct ShakeDetector {
    history: Vec<Movement>,
    shake_minimum_distance: i32,
    shake_interval_ms: u64,
    /// Shake factor in percent (e.g. 400 = distance must be 4× the diagonal).
    shake_factor: i32,
}

impl ShakeDetector {
    pub fn new(shake_minimum_distance: i32, shake_interval_ms: i32, shake_factor: i32) -> Self {
        Self {
            history: Vec::new(),
            shake_minimum_distance,
            shake_interval_ms: shake_interval_ms as u64,
            shake_factor,
        }
    }

    /// Update settings (can be called at runtime).
    pub fn update_settings(
        &mut self,
        shake_minimum_distance: i32,
        shake_interval_ms: i32,
        shake_factor: i32,
    ) {
        self.shake_minimum_distance = shake_minimum_distance;
        self.shake_interval_ms = shake_interval_ms as u64;
        self.shake_factor = shake_factor;
    }

    /// Feed a relative mouse movement and check for shake.
    /// Returns `true` if a shake was detected.
    pub fn feed(&mut self, dx: i64, dy: i64, tick: u64) -> bool {
        if dx == 0 && dy == 0 {
            return false;
        }

        if let Some(last) = self.history.last_mut() {
            // If moving in the same direction, merge into the last entry.
            if sign(last.dx) == sign(dx) && sign(last.dy) == sign(dy) {
                last.dx += dx;
                last.dy += dy;
                return false;
            }
        }

        // Direction changed (or first movement) — record and check.
        self.history.push(Movement { dx, dy, tick });

        // Only check on direction changes (when we have at least 2 movements).
        if self.history.len() >= 2 {
            return self.detect_shake(tick);
        }

        false
    }

    /// Clear movement history (called after a shake is detected or sonar stops).
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Core shake detection algorithm — mirrors C++ `DetectShake`.
    fn detect_shake(&mut self, now: u64) -> bool {
        let shake_start_tick = now.saturating_sub(self.shake_interval_ms);

        // Prune old movements.
        self.history.retain(|m| m.tick >= shake_start_tick);

        let mut distance_travelled: f64 = 0.0;
        let mut current_x: i64 = 0;
        let mut current_y: i64 = 0;
        let mut min_x: i64 = 0;
        let mut max_x: i64 = 0;
        let mut min_y: i64 = 0;
        let mut max_y: i64 = 0;

        for m in &self.history {
            current_x += m.dx;
            current_y += m.dy;
            distance_travelled +=
                ((m.dx as f64) * (m.dx as f64) + (m.dy as f64) * (m.dy as f64)).sqrt();
            min_x = min_x.min(current_x);
            max_x = max_x.max(current_x);
            min_y = min_y.min(current_y);
            max_y = max_y.max(current_y);
        }

        if distance_travelled < self.shake_minimum_distance as f64 {
            return false;
        }

        let rect_width = (max_x - min_x) as f64;
        let rect_height = (max_y - min_y) as f64;
        let diagonal = (rect_width * rect_width + rect_height * rect_height).sqrt();

        if diagonal > 0.0 && distance_travelled / diagonal > (self.shake_factor as f64 / 100.0) {
            self.history.clear();
            return true;
        }

        false
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Rapid back-and-forth horizontal shake triggers ──

    #[test]
    fn rapid_horizontal_shake_triggers() {
        let mut det = ShakeDetector::new(1000, 1000, 400);

        // Simulate rapid back-and-forth horizontal shake.
        // Total distance will be large, bounding rect will be narrow.
        let mut tick = 1000u64;
        let mut triggered = false;

        for i in 0..30 {
            let dx = if i % 2 == 0 { 200 } else { -200 };
            tick += 20;
            if det.feed(dx, 0, tick) {
                triggered = true;
                break;
            }
        }

        assert!(
            triggered,
            "Rapid horizontal shake should trigger detection"
        );
    }

    // ── Slow movement doesn't trigger ──

    #[test]
    fn slow_movement_does_not_trigger() {
        let mut det = ShakeDetector::new(1000, 1000, 400);

        // Move slowly in one direction — no direction changes.
        for i in 0..50 {
            let triggered = det.feed(10, 0, 1000 + i * 100);
            assert!(
                !triggered,
                "Slow unidirectional movement should not trigger"
            );
        }
    }

    // ── Circular motion doesn't trigger (no sharp reversals) ──

    #[test]
    fn circular_motion_does_not_trigger() {
        // Circular motion: distance/diagonal ratio ≈ 2.35 per cycle.
        // With shake_factor = 500 (5.0×) and 2 cycles ≈ 4.69, stays under threshold.
        let mut det = ShakeDetector::new(500, 1000, 500);

        let directions: &[(i64, i64)] = &[
            (100, 0),
            (70, 70),
            (0, 100),
            (-70, 70),
            (-100, 0),
            (-70, -70),
            (0, -100),
            (70, -70),
        ];

        let mut tick = 1000u64;
        for cycle in 0..2 {
            for &(dx, dy) in directions {
                tick += 20;
                let triggered = det.feed(dx, dy, tick);
                assert!(
                    !triggered,
                    "Circular motion should not trigger (cycle {cycle})"
                );
            }
        }
    }

    // ── Distance below minimum doesn't trigger ──

    #[test]
    fn distance_below_minimum_does_not_trigger() {
        // Set a very high minimum distance.
        let mut det = ShakeDetector::new(100_000, 1000, 400);

        let mut tick = 1000u64;
        for i in 0..20 {
            let dx = if i % 2 == 0 { 100 } else { -100 };
            tick += 20;
            let triggered = det.feed(dx, 0, tick);
            assert!(!triggered, "Should not trigger when total distance is below minimum");
        }
    }

    // ── Old movements are pruned ──

    #[test]
    fn old_movements_are_pruned() {
        let mut det = ShakeDetector::new(500, 1000, 400);

        // Add movements at tick=100..200
        det.feed(200, 0, 100);
        det.feed(-200, 0, 200);

        // Jump time forward by 2 seconds — all old movements should be pruned.
        // New movements alone don't have enough distance.
        let triggered = det.feed(50, 0, 2200);
        assert!(!triggered, "Old movements should be pruned");

        let triggered = det.feed(-50, 0, 2250);
        assert!(!triggered, "Pruned history should not contribute to detection");
    }

    // ── Same-direction movements are merged ──

    #[test]
    fn same_direction_movements_merged() {
        let mut det = ShakeDetector::new(1000, 1000, 400);

        // Multiple same-direction movements should be merged.
        det.feed(10, 0, 1000);
        det.feed(10, 0, 1010);
        det.feed(10, 0, 1020);
        // Only one entry in history (all same direction).
        // Internal check: feeding a direction change to see that we get
        // correct computation (only 2 entries: merged forward + one reverse).
        let triggered = det.feed(-10, 0, 1030);
        assert!(!triggered, "Small movements shouldn't trigger");
    }

    // ── Vertical shake also triggers ──

    #[test]
    fn rapid_vertical_shake_triggers() {
        let mut det = ShakeDetector::new(1000, 1000, 400);

        let mut tick = 1000u64;
        let mut triggered = false;

        for i in 0..30 {
            let dy = if i % 2 == 0 { 200 } else { -200 };
            tick += 20;
            if det.feed(0, dy, tick) {
                triggered = true;
                break;
            }
        }

        assert!(triggered, "Rapid vertical shake should trigger detection");
    }

    // ── Zero movement is ignored ──

    #[test]
    fn zero_movement_ignored() {
        let mut det = ShakeDetector::new(1000, 1000, 400);
        let triggered = det.feed(0, 0, 1000);
        assert!(!triggered, "Zero movement should be ignored");
    }

    // ── Clear resets state ──

    #[test]
    fn clear_resets_state() {
        let mut det = ShakeDetector::new(1000, 1000, 400);
        det.feed(200, 0, 1000);
        det.feed(-200, 0, 1020);
        det.clear();

        // After clearing, old movements should not count.
        let triggered = det.feed(50, 0, 1040);
        assert!(!triggered, "After clear, history should be empty");
    }

    // ── High shake factor requires more extreme shaking ──

    #[test]
    fn high_shake_factor_harder_to_trigger() {
        // Very high factor — hard to trigger.
        let mut det = ShakeDetector::new(1000, 1000, 10000);

        let mut tick = 1000u64;
        let mut triggered = false;

        for i in 0..30 {
            let dx = if i % 2 == 0 { 200 } else { -200 };
            tick += 20;
            if det.feed(dx, 0, tick) {
                triggered = true;
                break;
            }
        }

        assert!(
            !triggered,
            "Very high shake factor should make it harder to trigger"
        );
    }
}
