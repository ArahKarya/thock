//! Shared application state, accessed from the key listener, the tray, and
//! Tauri commands.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use crate::audio::AudioCmd;
use crate::config::Config;
use crate::soundpack::SoundPack;

pub struct AppShared {
    config: Mutex<Config>,
    pack: Mutex<SoundPack>,
    // `Sender` is `!Sync`; wrap it so `AppShared` can be shared behind an `Arc`.
    audio_tx: Mutex<Sender<AudioCmd>>,
    config_path: PathBuf,
    /// Set via the THOCK_DEBUG env var; logs each key event to stderr.
    debug: bool,
}

impl AppShared {
    pub fn new(
        config: Config,
        pack: SoundPack,
        audio_tx: Sender<AudioCmd>,
        config_path: PathBuf,
    ) -> Self {
        Self {
            config: Mutex::new(config),
            pack: Mutex::new(pack),
            audio_tx: Mutex::new(audio_tx),
            config_path,
            debug: std::env::var_os("THOCK_DEBUG").is_some(),
        }
    }

    /// A copy of the current config, safe to hand to the UI.
    pub fn snapshot(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    /// Display name of the currently loaded sound pack.
    pub fn pack_name(&self) -> String {
        self.pack.lock().unwrap().name.clone()
    }

    /// Mutate the config, persist it to disk, and return the new snapshot.
    pub fn update<F: FnOnce(&mut Config)>(&self, f: F) -> Config {
        let snapshot = {
            let mut cfg = self.config.lock().unwrap();
            f(&mut cfg);
            cfg.clamp();
            cfg.clone()
        };
        snapshot.save(&self.config_path);
        snapshot
    }

    /// Handle a key event from the OS listener. `logical` is the pack key name
    /// ("Space", "Enter", …) or "" for the pack default sound.
    pub fn on_key(&self, logical: &str, press: bool) {
        if self.debug {
            eprintln!("thock[debug]: key logical={logical:?} press={press}");
        }
        let (enabled, play_on_release) = {
            let cfg = self.config.lock().unwrap();
            (cfg.enabled, cfg.play_on_release)
        };
        if !enabled || (!press && !play_on_release) {
            return;
        }
        self.play_logical(logical, press);
    }

    /// Play a preview click (used by the Settings "Test" button).
    pub fn play_test(&self) {
        self.play_logical("", true);
    }

    fn play_logical(&self, logical: &str, press: bool) {
        let (volume, pitch) = {
            let cfg = self.config.lock().unwrap();
            (cfg.volume, cfg.pitch_variation)
        };
        let sound = match self.pack.lock().unwrap().resolve(logical, press) {
            Some(s) => s,
            None => return,
        };
        // +/- up to ~12% playback speed for natural variation.
        let jitter = (fastrand::f32() * 2.0 - 1.0) * pitch * 0.12;
        let speed = 1.0 + jitter;
        let _ = self.audio_tx.lock().unwrap().send(AudioCmd::Play {
            sound,
            volume,
            speed,
        });
    }
}
