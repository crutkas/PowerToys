use std::time::Instant;

/// Debounce logic for file-change notifications.
///
/// The C++ `FileWatcher` uses `wil::make_folder_change_reader_nothrow` to watch
/// for `FILE_NOTIFY_CHANGE_LAST_WRITE`. This module provides the
/// platform-independent debounce timer so it can be unit-tested without Win32.
///
/// Production code wraps this with the actual Win32 directory watcher.

/// Default debounce window matching PowerToys behaviour.
pub const DEFAULT_DEBOUNCE_MS: u64 = 200;

/// Pure-logic debounce timer.
pub struct DebouncedNotifier {
    debounce_ms: u64,
    last_event: Option<Instant>,
    pending: bool,
}

impl DebouncedNotifier {
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            debounce_ms,
            last_event: None,
            pending: false,
        }
    }

    /// Called when a raw file-change event arrives.
    /// Returns `true` if the callback should fire immediately (first event after
    /// the debounce window). Returns `false` if the event was coalesced.
    pub fn on_event(&mut self, now: Instant) -> bool {
        match self.last_event {
            None => {
                // First event ever — start the debounce window.
                self.last_event = Some(now);
                self.pending = true;
                false
            }
            Some(last) => {
                let elapsed = now.duration_since(last).as_millis() as u64;
                self.last_event = Some(now);
                if elapsed >= self.debounce_ms && self.pending {
                    // Window expired and we have a pending notification.
                    self.pending = false;
                    true
                } else {
                    self.pending = true;
                    false
                }
            }
        }
    }

    /// Check whether a pending notification should fire (called from a timer
    /// or poll loop). Returns `true` if the debounce window has elapsed since
    /// the last raw event and there is a pending notification.
    pub fn poll(&mut self, now: Instant) -> bool {
        if !self.pending {
            return false;
        }
        if let Some(last) = self.last_event {
            let elapsed = now.duration_since(last).as_millis() as u64;
            if elapsed >= self.debounce_ms {
                self.pending = false;
                return true;
            }
        }
        false
    }

    /// Whether there is a pending notification waiting for the debounce window.
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// Reset internal state.
    pub fn reset(&mut self) {
        self.last_event = None;
        self.pending = false;
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn first_event_does_not_fire() {
        let mut d = DebouncedNotifier::new(200);
        let now = Instant::now();
        assert!(!d.on_event(now));
        assert!(d.is_pending());
    }

    #[test]
    fn rapid_events_coalesce() {
        let mut d = DebouncedNotifier::new(200);
        let t0 = Instant::now();
        d.on_event(t0);
        // Rapid second event 50ms later — should coalesce.
        assert!(!d.on_event(t0 + Duration::from_millis(50)));
        // Another at 100ms — still coalesced.
        assert!(!d.on_event(t0 + Duration::from_millis(100)));
        assert!(d.is_pending());
    }

    #[test]
    fn fires_after_debounce_window() {
        let mut d = DebouncedNotifier::new(200);
        let t0 = Instant::now();
        d.on_event(t0);
        // Poll before window — should not fire.
        assert!(!d.poll(t0 + Duration::from_millis(100)));
        // Poll after window — should fire.
        assert!(d.poll(t0 + Duration::from_millis(250)));
        assert!(!d.is_pending());
    }

    #[test]
    fn poll_does_not_fire_twice() {
        let mut d = DebouncedNotifier::new(200);
        let t0 = Instant::now();
        d.on_event(t0);
        assert!(d.poll(t0 + Duration::from_millis(250)));
        // Second poll should not fire again.
        assert!(!d.poll(t0 + Duration::from_millis(300)));
    }

    #[test]
    fn on_event_fires_after_gap() {
        let mut d = DebouncedNotifier::new(200);
        let t0 = Instant::now();
        d.on_event(t0);
        // After debounce window, next on_event should fire.
        assert!(d.on_event(t0 + Duration::from_millis(300)));
    }

    #[test]
    fn reset_clears_state() {
        let mut d = DebouncedNotifier::new(200);
        let t0 = Instant::now();
        d.on_event(t0);
        d.reset();
        assert!(!d.is_pending());
        // After reset, first event does not fire.
        assert!(!d.on_event(t0 + Duration::from_millis(500)));
    }

    #[test]
    fn custom_debounce_window() {
        let mut d = DebouncedNotifier::new(500);
        let t0 = Instant::now();
        d.on_event(t0);
        // At 300ms — still within 500ms window.
        assert!(!d.poll(t0 + Duration::from_millis(300)));
        // At 600ms — past window.
        assert!(d.poll(t0 + Duration::from_millis(600)));
    }
}
