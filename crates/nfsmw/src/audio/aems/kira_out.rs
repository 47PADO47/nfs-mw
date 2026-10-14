//! The sample layer's output through `kira`.

use std::time::Duration;

use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{PlaybackRate, Tween};

use super::host::{Sampler, Voice, VoiceEffects, VoiceStatus};
use crate::audio::volume::decibels;
use crate::audio::{Audio, Group};

/// Volume and pitch changes run over this long (two module ticks), so a step is not heard as a click.
const SMOOTH: Duration = Duration::from_millis(33);
/// A voice that stops fades out over this long.
const FADE_OUT: Duration = Duration::from_millis(40);

fn tween(duration: Duration) -> Tween {
    Tween { duration, ..Tween::default() }
}

/// The game's sound output as a [`Sampler`]: voices go to the engine group.
pub struct KiraSampler<'a>(pub &'a mut Audio);
pub struct EffectSampler<'a>(pub &'a mut Audio, pub Group);

struct KiraVoice {
    handle: StaticSoundHandle,
    looping: bool,
    seconds: f64,
    filters: Option<(crate::audio::filter::CutoffHandle, crate::audio::filter::CutoffHandle)>,
    effects: Option<VoiceEffects>,
}

impl Sampler for KiraSampler<'_> {
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String> {
        self.0.bank_sound(bank, index)
    }

    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
        start(self.0, Group::Engine, data, gain, rate)
    }
}

impl Sampler for EffectSampler<'_> {
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String> {
        self.0.bank_sound(bank, index)
    }
    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
        start(self.0, self.1, data, gain, rate)
    }
}

fn start(audio: &mut Audio, group: Group, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
    let (looping, seconds) = (data.settings.loop_region.is_some(), data.duration().as_secs_f64());
    let data = data.volume(decibels(gain)).playback_rate(f64::from(rate));
    let shaped = match audio.play_shaped(group, data) {
        Ok(handle) => handle,
        Err(e) => {
            log::debug!("sample layer: {e}");
            return None;
        }
    };
    Some(Box::new(KiraVoice {
        handle: shaped.voice,
        looping,
        seconds,
        filters: Some((shaped.low, shaped.high)),
        effects: None,
    }))
}

impl Voice for KiraVoice {
    fn set_effects(&mut self, effects: VoiceEffects) {
        if self.effects == Some(effects) {
            return;
        }
        self.handle.set_panning(crate::audio::shaped::pan(effects.azimuth), tween(SMOOTH));
        if let Some((low, high)) = &mut self.filters {
            crate::audio::shaped::set_filters(low, high, effects.low_pass, effects.high_pass);
        }
        self.effects = Some(effects);
    }
    fn set_gain(&mut self, gain: f32) {
        self.handle.set_volume(decibels(gain), tween(SMOOTH));
    }

    fn set_rate(&mut self, rate: f32) {
        self.handle.set_playback_rate(PlaybackRate(f64::from(rate)), tween(SMOOTH));
    }

    fn pause(&mut self) {
        self.handle.pause(tween(SMOOTH));
    }

    fn resume(&mut self) {
        self.handle.resume(tween(SMOOTH));
    }

    fn stop(&mut self) {
        self.handle.stop(tween(FADE_OUT));
    }

    fn status(&self) -> VoiceStatus {
        let position = self.handle.position();
        let playing = self.handle.state() != PlaybackState::Stopped;
        let remaining = if self.looping { 0.0 } else { (self.seconds - position).max(0.0) };
        VoiceStatus { playing, elapsed_ms: (position * 1000.0) as i32, remaining_ms: (remaining * 1000.0) as i32 }
    }
}

impl Drop for KiraVoice {
    fn drop(&mut self) {
        self.handle.stop(tween(FADE_OUT));
    }
}
