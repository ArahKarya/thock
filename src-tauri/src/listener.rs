//! Global keyboard listener thread.
//!
//! Platform backends:
//! - **macOS**: a hand-rolled listen-only `CGEventTap`. We deliberately avoid
//!   `rdev` here: its macOS backend translates keycodes to strings via
//!   TIS/HIToolbox APIs from a background thread, which aborts the process on
//!   modern macOS (`dispatch_assert_queue`). We only need raw keycodes.
//!   Requires the Input Monitoring permission.
//! - **Linux (X11) / Windows**: `rdev`. OS key auto-repeat is suppressed by
//!   tracking held keys, so a held key clicks once.

use std::sync::Arc;
use std::thread;

use crate::state::AppShared;

pub fn spawn(shared: Arc<AppShared>) {
    thread::spawn(move || platform::run(shared));
}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::sync::Arc;

    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        EventField,
    };

    use crate::state::AppShared;

    // Missing from the core-graphics bindings; used to trigger the standard
    // macOS Input Monitoring permission prompt before creating the tap.
    extern "C" {
        fn CGRequestListenEventAccess() -> bool;
    }

    /// macOS virtual keycodes for the keys with dedicated pack sounds.
    fn logical_from_keycode(code: i64) -> &'static str {
        match code {
            36 | 76 => "Enter",
            48 => "Tab",
            49 => "Space",
            51 => "Backspace",
            54 | 55 => "Meta",
            56 | 60 => "Shift",
            58 | 61 => "Alt",
            59 | 62 => "Control",
            _ => "",
        }
    }

    /// Modifier keycodes only ever arrive as `FlagsChanged` events.
    fn is_modifier(code: i64) -> bool {
        matches!(code, 54..=63)
    }

    pub fn run(shared: Arc<AppShared>) {
        let granted = unsafe { CGRequestListenEventAccess() };
        if !granted {
            eprintln!(
                "thock: Input Monitoring permission not granted. Enable it in \
                 System Settings > Privacy & Security > Input Monitoring, then \
                 relaunch Thock."
            );
            // Fall through: tap creation below fails cleanly without it.
        }

        // Modifiers have no distinct press/release event type, only
        // FlagsChanged; track which are down to derive the phase.
        let held_modifiers: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());

        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::TailAppendEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::KeyDown,
                CGEventType::KeyUp,
                CGEventType::FlagsChanged,
            ],
            move |_proxy, event_type, event| {
                let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                match event_type {
                    CGEventType::KeyDown => {
                        let autorepeat =
                            event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT);
                        if autorepeat == 0 {
                            shared.on_key(logical_from_keycode(code), true);
                        }
                    }
                    CGEventType::KeyUp => {
                        shared.on_key(logical_from_keycode(code), false);
                    }
                    CGEventType::FlagsChanged if is_modifier(code) => {
                        let press = held_modifiers.borrow_mut().insert(code);
                        if !press {
                            held_modifiers.borrow_mut().remove(&code);
                        }
                        shared.on_key(logical_from_keycode(code), press);
                    }
                    _ => {}
                }
                None
            },
        );

        let tap = match tap {
            Ok(tap) => tap,
            Err(()) => {
                eprintln!(
                    "thock: could not create the keyboard event tap (missing \
                     Input Monitoring permission?). Sounds will stay silent."
                );
                return;
            }
        };

        unsafe {
            let source = match tap.mach_port.create_runloop_source(0) {
                Ok(source) => source,
                Err(()) => {
                    eprintln!("thock: could not create run-loop source for the event tap");
                    return;
                }
            };
            CFRunLoop::get_current().add_source(&source, kCFRunLoopCommonModes);
            tap.enable();
            CFRunLoop::run_current();
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    use rdev::{listen, Event, EventType};

    use crate::keymap;
    use crate::state::AppShared;

    pub fn run(shared: Arc<AppShared>) {
        let held: Mutex<HashSet<rdev::Key>> = Mutex::new(HashSet::new());

        let callback = move |event: Event| match event.event_type {
            EventType::KeyPress(key) => {
                let is_new = held.lock().unwrap().insert(key);
                if is_new {
                    shared.on_key(keymap::logical_name(key), true);
                }
            }
            EventType::KeyRelease(key) => {
                held.lock().unwrap().remove(&key);
                shared.on_key(keymap::logical_name(key), false);
            }
            _ => {}
        };

        if let Err(err) = listen(callback) {
            eprintln!(
                "thock: keyboard listener failed: {err:?}. On Linux, global \
                 capture needs X11 (Wayland is restricted)."
            );
        }
    }
}
