//! Sound-pack model and loader.
//!
//! All bundled packs are embedded in the binary so the app works with zero
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

use serde::{Deserialize, Serialize};

/// Audio bytes (a decoded-on-play WAV buffer), cheap to clone.
pub type SoundBytes = Arc<[u8]>;

#[derive(Clone, Default)]
pub struct KeySound {
    pub press: Option<SoundBytes>,
    pub release: Option<SoundBytes>,
}

#[derive(Clone)]
pub struct SoundPack {
    pub id: String,
    pub name: String,
    default: KeySound,
    keys: HashMap<String, KeySound>,
}

#[derive(Clone, Serialize)]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl SoundPack {
    /// Resolve the sound for a logical key name and press/release phase,
    /// falling back to the pack default when the key has no specific override.
    pub fn resolve(&self, logical: &str, press: bool) -> Option<SoundBytes> {
        self.resolve_exact(logical, press)
            .or_else(|| pick(&self.default, press))
    }

    /// Like [`resolve`], but without the default fallback. Used for mouse
    /// buttons, which should stay silent in packs that define no mouse sounds.
    pub fn resolve_exact(&self, logical: &str, press: bool) -> Option<SoundBytes> {
        self.keys.get(logical).and_then(|k| pick(k, press))
    }

    /// Load an embedded pack by id; falls back to the first bundled pack.
    pub fn load(id: &str) -> Self {
        let embedded = EMBEDDED.iter().find(|p| p.id == id).unwrap_or(&EMBEDDED[0]);
        let manifest: RawPack =
            serde_json::from_slice(embedded.manifest).expect("bundled pack.json is valid");

        let file_bytes = |name: &Option<String>| -> Option<SoundBytes> {
            let wanted = name.as_deref()?;
            let (_, bytes) = embedded.files.iter().find(|(f, _)| *f == wanted)?;
            Some(Arc::from(bytes.to_vec().into_boxed_slice()) as SoundBytes)
        };
        let to_key_sound = |raw: &RawKey| KeySound {
            press: file_bytes(&raw.press),
            release: file_bytes(&raw.release),
        };

        SoundPack {
            id: embedded.id.to_string(),
            name: manifest.name,
            default: to_key_sound(&manifest.default),
            keys: manifest
                .keys
                .iter()
                .map(|(k, v)| (k.clone(), to_key_sound(v)))
                .collect(),
        }
    }

    /// All bundled packs, in menu order.
    pub fn list() -> Vec<PackInfo> {
        EMBEDDED
            .iter()
            .map(|p| {
                let manifest: RawPack =
                    serde_json::from_slice(p.manifest).expect("bundled pack.json is valid");
                PackInfo {
                    id: p.id.to_string(),
                    name: manifest.name,
                    description: manifest.description,
                }
            })
            .collect()
    }
}

fn pick(k: &KeySound, press: bool) -> Option<SoundBytes> {
    if press {
        k.press.clone()
    } else {
        k.release.clone()
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
    #[serde(default)]
    description: String,
    default: RawKey,
    #[serde(default)]
    keys: HashMap<String, RawKey>,
}

struct EmbeddedPack {
    id: &'static str,
    manifest: &'static [u8],
    files: &'static [(&'static str, &'static [u8])],
}

macro_rules! embed_pack {
    ($id:literal) => {
        EmbeddedPack {
            id: $id,
            manifest: include_bytes!(concat!("../sounds/", $id, "/pack.json")),
            files: &[
                (
                    "key_press.wav",
                    include_bytes!(concat!("../sounds/", $id, "/key_press.wav")),
                ),
                (
                    "key_release.wav",
                    include_bytes!(concat!("../sounds/", $id, "/key_release.wav")),
                ),
                (
                    "space_press.wav",
                    include_bytes!(concat!("../sounds/", $id, "/space_press.wav")),
                ),
                (
                    "space_release.wav",
                    include_bytes!(concat!("../sounds/", $id, "/space_release.wav")),
                ),
                (
                    "enter_press.wav",
                    include_bytes!(concat!("../sounds/", $id, "/enter_press.wav")),
                ),
                (
                    "enter_release.wav",
                    include_bytes!(concat!("../sounds/", $id, "/enter_release.wav")),
                ),
                (
                    "backspace_press.wav",
                    include_bytes!(concat!("../sounds/", $id, "/backspace_press.wav")),
                ),
                (
                    "modifier_press.wav",
                    include_bytes!(concat!("../sounds/", $id, "/modifier_press.wav")),
                ),
                (
                    "mouse_press.wav",
                    include_bytes!(concat!("../sounds/", $id, "/mouse_press.wav")),
                ),
                (
                    "mouse_release.wav",
                    include_bytes!(concat!("../sounds/", $id, "/mouse_release.wav")),
                ),
            ],
        }
    };
}

/// Bundled packs. The first entry is the fallback for unknown pack ids.
static EMBEDDED: &[EmbeddedPack] = &[
    embed_pack!("thock"),
    embed_pack!("clicky"),
    embed_pack!("tactile"),
    embed_pack!("typewriter"),
];
