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
pub struct DelayedSampler<'a>(pub &'a mut Audio, pub Group, pub f32);

struct KiraVoice {
    handle: StaticSoundHandle,
    looping: bool,
    seconds: f64,
    filters: Option<(crate::audio::filter::CutoffHandle, crate::audio::filter::CutoffHandle)>,
    effects: Option<VoiceEffects>,
    channels: u16,
    gain: f32,
}

impl Sampler for KiraSampler<'_> {
    fn channels(&self, bank: &str, index: usize) -> u16 {
        self.0.sound_channels.get(&(bank.to_owned(), index)).copied().unwrap_or(1)
    }
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String> {
        self.0.bank_sound(bank, index)
    }

    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
        start(self.0, Group::Engine, data, gain, rate)
    }
}

impl Sampler for EffectSampler<'_> {
    fn channels(&self, bank: &str, index: usize) -> u16 {
        self.0.sound_channels.get(&(bank.to_owned(), index)).copied().unwrap_or(1)
    }
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String> {
        self.0.bank_sound(bank, index)
    }
    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
        start(self.0, self.1, data, gain, rate)
    }
}

impl Sampler for DelayedSampler<'_> {
    fn channels(&self, bank: &str, index: usize) -> u16 {
        self.0.sound_channels.get(&(bank.to_owned(), index)).copied().unwrap_or(1)
    }
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String> {
        self.0.bank_sound(bank, index)
    }
    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
        let data = data.start_time(kira::StartTime::Delayed(Duration::from_secs_f32(self.2.max(0.0))));
        start(self.0, self.1, data, gain, rate)
    }
}

fn start(audio: &mut Audio, group: Group, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>> {
    let (looping, seconds) = (data.settings.loop_region.is_some(), data.duration().as_secs_f64());
    let data = data.volume(decibels(gain * std::f32::consts::FRAC_1_SQRT_2)).playback_rate(f64::from(rate));
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
        channels: 1,
        gain,
    }))
}

impl Voice for KiraVoice {
    fn set_channels(&mut self, channels: u16) {
        self.channels = channels;
    }
    fn set_effects(&mut self, effects: VoiceEffects) {
        if self.effects == Some(effects) {
            return;
        }
        let smooth = if self.effects.is_none() { Duration::ZERO } else { SMOOTH };
        if self.channels == 1 {
            self.handle.set_panning(crate::audio::shaped::pan(effects.azimuth), tween(smooth));
        }
        if let Some((low, high)) = &mut self.filters {
            crate::audio::shaped::set_filters(low, high, effects.low_pass, effects.high_pass);
        }
        self.effects = Some(effects);
        self.apply_gain(smooth);
    }
    fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
        self.apply_gain(SMOOTH);
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

impl KiraVoice {
    fn apply_gain(&mut self, smooth: Duration) {
        let dry = self.effects.map_or(1.0, |fx| (((fx.dry * 32767.0).round() as u32) >> 8) as f32 / 127.0);
        let normalize = if self.channels == 1 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
        self.handle.set_volume(decibels(self.gain * dry * normalize), tween(smooth));
    }
}

impl Drop for KiraVoice {
    fn drop(&mut self) {
        self.handle.stop(tween(FADE_OUT));
    }
}
