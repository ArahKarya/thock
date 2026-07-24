//! Audio output thread.
//!
//! rodio's `OutputStream` is not `Send`, so it lives entirely on one dedicated
//! thread. Other threads submit `Play` commands over a channel. Short clips are
//! decoded on play and mixed by rodio, allowing overlapping keystrokes.

use std::io::Cursor;
use std::sync::mpsc::{self, Sender};
use std::thread;

use rodio::{Decoder, OutputStream, Source};

use crate::soundpack::SoundBytes;

pub enum AudioCmd {
    Play {
        sound: SoundBytes,
        volume: f32,
        speed: f32,
    },
}

/// Spawn the audio thread and return a sender for submitting playback commands.
pub fn spawn() -> Sender<AudioCmd> {
    let (tx, rx) = mpsc::channel::<AudioCmd>();

    thread::spawn(move || {
        // Keep `_stream` alive for the lifetime of the thread; dropping it
        // closes the audio device.
        let (_stream, handle) = match OutputStream::try_default() {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("thock: could not open audio output: {err}");
                return;
            }
        };

        for cmd in rx {
            match cmd {
                AudioCmd::Play {
                    sound,
                    volume,
                    speed,
                } => {
                    let cursor = Cursor::new(sound.to_vec());
                    match Decoder::new(cursor) {
                        Ok(decoder) => {
                            let source = decoder
                                .convert_samples::<f32>()
                                .amplify(volume)
                                .speed(speed);
                            if let Err(err) = handle.play_raw(source) {
                                eprintln!("thock: playback error: {err}");
                            }
                        }
                        Err(err) => eprintln!("thock: decode error: {err}"),
                    }
                }
            }
        }
    });

    tx
}
