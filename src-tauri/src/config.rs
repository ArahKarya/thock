//! Persisted user settings.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Master on/off switch for playing sounds.
    pub enabled: bool,
    /// Output volume, 0.0..=1.0.
    pub volume: f32,
    /// Random pitch variation, 0.0..=1.0 (mapped to +/- ~12% playback speed).
    pub pitch_variation: f32,
    /// Whether to also play a sound on key release.
    pub play_on_release: bool,
    /// Whether mouse button clicks also play sounds.
    pub mouse_enabled: bool,
    /// Active sound pack id (see the bundled packs in `sounds/`).
    pub pack: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.6,
            pitch_variation: 0.25,
            play_on_release: true,
            mouse_enabled: true,
            pack: "thock".to_string(),
        }
    }
}

impl Config {
    pub fn clamp(&mut self) {
        self.volume = self.volume.clamp(0.0, 1.0);
        self.pitch_variation = self.pitch_variation.clamp(0.0, 1.0);
    }

    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Config>(&text) {
                Ok(mut cfg) => {
                    cfg.clamp();
                    cfg
                }
                Err(err) => {
                    eprintln!("thock: invalid config, using defaults: {err}");
                    Config::default()
                }
            },
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(err) = fs::write(path, text) {
                    eprintln!("thock: failed to save config: {err}");
                }
            }
            Err(err) => eprintln!("thock: failed to serialize config: {err}"),
        }
    }
}

/// Resolve the on-disk config file location for the app config dir.
pub fn config_file(config_dir: &Path) -> PathBuf {
    config_dir.join("config.json")
}
