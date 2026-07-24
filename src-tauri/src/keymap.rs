//! Map raw OS keys to the logical names used by sound packs.
//!
//! Anything not listed here falls back to the pack's `default` sound.

use rdev::Key;

pub fn logical_name(key: Key) -> &'static str {
    match key {
        Key::Return | Key::KpReturn => "Enter",
        Key::Space => "Space",
        Key::Backspace => "Backspace",
        Key::Tab => "Tab",
        Key::ShiftLeft | Key::ShiftRight => "Shift",
        Key::ControlLeft | Key::ControlRight => "Control",
        Key::Alt | Key::AltGr => "Alt",
        Key::MetaLeft | Key::MetaRight => "Meta",
        // Everything else uses the pack default.
        _ => "",
    }
}
