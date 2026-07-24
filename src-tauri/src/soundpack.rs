//! Sound-pack model and loader.
//!
//! The default pack is embedded in the binary so the app works with zero
//! external files. The manifest format (`pack.json`) is intentionally simple:
//!
//! ```json
//! {
//!   "name": "...",
//!   "default": { "press": "a.wav", "release": "b.wav" },
//!   "keys": { "Space": { "press": "space.wav", "release": "space_up.wav" } }
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;

/// Audio bytes (a decoded-on-play WAV/OGG buffer), cheap to clone.
pub type SoundBytes = Arc<[u8]>;

#[derive(Clone, Default)]
pub struct KeySound {
    pub press: Option<SoundBytes>,
    pub release: Option<SoundBytes>,
}

#[derive(Clone)]
pub struct SoundPack {
    pub name: String,
    default: KeySound,
    keys: HashMap<String, KeySound>,
}

impl SoundPack {
    /// Resolve the sound to play for a logical key name and press/release phase,
    /// falling back to the pack default when a key has no specific override.
    pub fn resolve(&self, logical: &str, press: bool) -> Option<SoundBytes> {
        let specific = self.keys.get(logical);
        let pick = |k: &KeySound| {
            if press {
                k.press.clone()
            } else {
                k.release.clone()
            }
        };

        specific.and_then(pick).or_else(|| pick(&self.default))
    }

    /// Load the bundled default pack from bytes embedded at compile time.
    pub fn load_default() -> Self {
        let manifest: RawPack =
            serde_json::from_slice(EMBEDDED_MANIFEST).expect("bundled pack.json is valid");

        let resolve_bytes = |name: &Option<String>| -> Option<SoundBytes> {
            name.as_deref()
                .and_then(embedded_file)
                .map(|b| Arc::from(b.to_vec().into_boxed_slice()) as SoundBytes)
        };
        let to_key_sound = |raw: &RawKey| KeySound {
            press: resolve_bytes(&raw.press),
            release: resolve_bytes(&raw.release),
        };

        let keys = manifest
            .keys
            .iter()
            .map(|(k, v)| (k.clone(), to_key_sound(v)))
            .collect();

        SoundPack {
            name: manifest.name,
            default: to_key_sound(&manifest.default),
            keys,
        }
    }
}

#[derive(Deserialize)]
struct RawKey {
    #[serde(default)]
    press: Option<String>,
    #[serde(default)]
    release: Option<String>,
}

#[derive(Deserialize)]
struct RawPack {
    name: String,
    default: RawKey,
    #[serde(default)]
    keys: HashMap<String, RawKey>,
}

static EMBEDDED_MANIFEST: &[u8] = include_bytes!("../sounds/thock/pack.json");

/// Map a filename referenced by the bundled manifest to its embedded bytes.
fn embedded_file(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "key_press.wav" => include_bytes!("../sounds/thock/key_press.wav"),
        "key_release.wav" => include_bytes!("../sounds/thock/key_release.wav"),
        "space_press.wav" => include_bytes!("../sounds/thock/space_press.wav"),
        "space_release.wav" => include_bytes!("../sounds/thock/space_release.wav"),
        "enter_press.wav" => include_bytes!("../sounds/thock/enter_press.wav"),
        "enter_release.wav" => include_bytes!("../sounds/thock/enter_release.wav"),
        "backspace_press.wav" => include_bytes!("../sounds/thock/backspace_press.wav"),
        "modifier_press.wav" => include_bytes!("../sounds/thock/modifier_press.wav"),
        _ => return None,
    })
}
