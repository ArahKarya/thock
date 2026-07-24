//! Global keyboard listener thread.
//!
//! Uses `rdev` to capture key events system-wide. OS key auto-repeat (holding a
//! key) is suppressed by tracking currently-held keys, so a held key clicks once.
//!
//! Platform notes:
//! - macOS: requires Input Monitoring / Accessibility permission.
//! - Linux (X11): needs an X server; Wayland global capture is limited.
//! - Windows: uses a low-level keyboard hook, no extra permission.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

use rdev::{listen, Event, EventType};

use crate::state::AppShared;

pub fn spawn(shared: Arc<AppShared>) {
    thread::spawn(move || {
        let held: Mutex<HashSet<rdev::Key>> = Mutex::new(HashSet::new());

        let callback = move |event: Event| match event.event_type {
            EventType::KeyPress(key) => {
                let is_new = held.lock().unwrap().insert(key);
                if is_new {
                    shared.on_key(key, true);
                }
            }
            EventType::KeyRelease(key) => {
                held.lock().unwrap().remove(&key);
                shared.on_key(key, false);
            }
            _ => {}
        };

        if let Err(err) = listen(callback) {
            eprintln!(
                "thock: keyboard listener failed: {err:?}. \
                 On macOS, grant Input Monitoring permission in \
                 System Settings > Privacy & Security."
            );
        }
    });
}
