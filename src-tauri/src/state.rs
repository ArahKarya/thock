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

    /// Switch the active sound pack, persist the choice, and return the new
    /// config snapshot.
    pub fn set_pack(&self, id: &str) -> Config {
        let pack = SoundPack::load(id);
        let loaded_id = pack.id.clone();
        *self.pack.lock().unwrap() = pack;
        self.update(|c| c.pack = loaded_id)
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

    /// Handle a mouse-button event from the OS listener. `logical` is
    /// "MouseLeft" / "MouseRight" / "MouseMiddle". Both press and release play
    /// (a physical click is a down-up pair), independent of `play_on_release`.
    pub fn on_mouse(&self, logical: &str, press: bool) {
        if self.debug {
            eprintln!("thock[debug]: mouse logical={logical:?} press={press}");
        }
        let (enabled, mouse_enabled) = {
            let cfg = self.config.lock().unwrap();
            (cfg.enabled, cfg.mouse_enabled)
        };
        if !enabled || !mouse_enabled {
            return;
        }
        // Exact resolution: packs without mouse sounds stay silent instead of
        // clicking with the keyboard default.
        let sound = self.pack.lock().unwrap().resolve_exact(logical, press);
        if let Some(sound) = sound {
            self.send_play(sound);
        }
    }

    /// Play a preview click (used by the Settings "Test" button).
    pub fn play_test(&self) {
        self.play_logical("", true);
    }

    fn play_logical(&self, logical: &str, press: bool) {
        let sound = match self.pack.lock().unwrap().resolve(logical, press) {
            Some(s) => s,
            None => return,
        };
        self.send_play(sound);
    }

    fn send_play(&self, sound: crate::soundpack::SoundBytes) {
        let (volume, pitch) = {
            let cfg = self.config.lock().unwrap();
            (cfg.volume, cfg.pitch_variation)
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
