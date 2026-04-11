//! Named event creation and signaling.
//!
//! Wraps Win32 `CreateEventW`, `SetEvent`, `ResetEvent`, `WaitForSingleObject`.
//! PowerToys modules communicate via named events (e.g., terminate, toggle).

use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForSingleObject,
};

use crate::string::to_wide;

/// RAII wrapper around a Win32 named event.
pub struct NamedEvent {
    handle: HANDLE,
}

// SAFETY: Win32 event handles can be sent across threads.
unsafe impl Send for NamedEvent {}
unsafe impl Sync for NamedEvent {}

impl NamedEvent {
    /// Create or open a named event. `manual_reset = true` creates a manual-reset event.
    pub fn create(name: &str, manual_reset: bool) -> Option<Self> {
        let wide = to_wide(name);
        let handle = unsafe {
            CreateEventW(
                ptr::null(),
                manual_reset as i32,
                0, // initial state: non-signaled
                wide.as_ptr(),
            )
        };
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Create an unnamed auto-reset event.
    pub fn create_anonymous() -> Option<Self> {
        let handle = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
        if handle.is_null() { None } else { Some(Self { handle }) }
    }

    /// Signal the event.
    pub fn set(&self) -> bool {
        unsafe { SetEvent(self.handle) != 0 }
    }

    /// Reset the event to non-signaled.
    pub fn reset(&self) -> bool {
        unsafe { ResetEvent(self.handle) != 0 }
    }

    /// Wait for the event to be signaled. Returns true if signaled, false on timeout.
    pub fn wait(&self, timeout_ms: u32) -> bool {
        unsafe { WaitForSingleObject(self.handle, timeout_ms) == WAIT_OBJECT_0 }
    }

    /// Wait indefinitely for the event to be signaled.
    pub fn wait_infinite(&self) -> bool {
        self.wait(0xFFFFFFFF) // INFINITE
    }

    /// Raw handle for use with `MsgWaitForMultipleObjects` or other APIs.
    pub fn raw(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for NamedEvent {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle); }
    }
}

/// Wait for any one of multiple handles to be signaled. Returns the index of the
/// signaled handle, or `None` on timeout.
pub fn wait_any(handles: &[HANDLE], timeout_ms: u32) -> Option<usize> {
    if handles.is_empty() {
        return None;
    }
    let result = unsafe {
        WaitForSingleObject(handles[0], 0) // placeholder — use MsgWait for multi
    };
    // For multi-handle waiting, callers should use MsgWaitForMultipleObjects directly.
    // This is a convenience for the single-handle case.
    if handles.len() == 1 {
        let r = unsafe { WaitForSingleObject(handles[0], timeout_ms) };
        if r == WAIT_OBJECT_0 { Some(0) } else { None }
    } else {
        // Multi-handle: use WaitForMultipleObjects
        let r = unsafe {
            windows_sys::Win32::System::Threading::WaitForMultipleObjects(
                handles.len() as u32,
                handles.as_ptr(),
                0, // wait for ANY
                timeout_ms,
            )
        };
        let _ = result;
        if r >= WAIT_OBJECT_0 && r < WAIT_OBJECT_0 + handles.len() as u32 {
            Some((r - WAIT_OBJECT_0) as usize)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_signal() {
        let evt = NamedEvent::create_anonymous().expect("create event");
        assert!(!evt.wait(0)); // not signaled yet
        assert!(evt.set());
        assert!(evt.wait(0)); // now signaled
    }

    #[test]
    fn manual_reset_event() {
        let evt = NamedEvent::create("Local\\powertoys_win32_test_manual", true).unwrap();
        evt.set();
        assert!(evt.wait(0));
        assert!(evt.wait(0)); // still signaled (manual reset)
        evt.reset();
        assert!(!evt.wait(0)); // now non-signaled
    }

    #[test]
    fn named_event_shared() {
        let name = "Local\\powertoys_win32_test_shared";
        let evt1 = NamedEvent::create(name, false).unwrap();
        let evt2 = NamedEvent::create(name, false).unwrap();
        evt1.set();
        assert!(evt2.wait(100)); // second handle sees the signal
    }
}
