//! Where the sound goes: the speech track of the mixer, behind a trait so the speech logic can be tested without a
//! sound device.

use ea_audio::Pcm;
use kira::Tween;
use kira::sound::PlaybackState;
use kira::sound::static_sound::StaticSoundHandle;
use kira::track::TrackHandle;

use crate::audio::{pcm, volume};

pub trait Sink {
    /// Whether a line is sounding.
    fn busy(&self) -> bool;

    /// Start a line (the one in progress, if any, is not touched: `stop` it first).
    fn play(&mut self, pcm: &Pcm) -> Result<(), String>;

    /// Fade out the line in progress.
    fn stop(&mut self);

    /// Linear volume of the speech track.
    fn set_volume(&mut self, amplitude: f32);
}

/// The speech track of the `kira` mixer. Without a sound device it has no track and nothing can be said.
pub struct KiraSink {
    track: Option<TrackHandle>,
    handle: Option<StaticSoundHandle>,
}

impl KiraSink {
    pub fn new(track: Option<TrackHandle>) -> Self {
        Self { track, handle: None }
    }
}

impl Sink for KiraSink {
    fn busy(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| h.state() != PlaybackState::Stopped)
    }

    fn play(&mut self, data: &Pcm) -> Result<(), String> {
        let track = self.track.as_mut().ok_or("there is no sound device")?;
        self.handle = Some(track.play(pcm::sound(data)).map_err(|e| e.to_string())?);
        Ok(())
    }

    fn stop(&mut self) {
        let Some(mut handle) = self.handle.take() else { return };
        handle.stop(Tween::default());
    }

    fn set_volume(&mut self, amplitude: f32) {
        let Some(track) = self.track.as_mut() else { return };
        track.set_volume(volume::decibels(amplitude), Tween::default());
    }
}
