//! What the AEMS modules ask of the sound output: bank sounds played as voices.

use blackbox_aems::{ClassCall, Host, PlayerInputs, SampleEntry, VoiceState, input};
use kira::sound::static_sound::StaticSoundData;

/// How a voice is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceStatus {
    pub playing: bool,
    pub elapsed_ms: i32,
    /// 0 for a sound that loops.
    pub remaining_ms: i32,
}

/// One playing sound. Dropping it ends the sound.
pub trait Voice {
    fn set_gain(&mut self, gain: f32);
    /// The playback rate, 1 = natural.
    fn set_rate(&mut self, rate: f32);
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn status(&self) -> VoiceStatus;
}

/// The sound output the modules play through.
pub trait Sampler {
    /// Bank sound `index` (counted as in the bank's `BNKl`) of the bank at `bank`, relative to `SOUND/`.
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String>;
    /// Starts `data` at `gain` (linear) and playback rate `rate`.
    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>>;
}

/// The slowest playback the voices are asked for (an input of 0 would stop time).
const MIN_RATE: f32 = 0.01;

/// The voices and the object of one running module.
#[derive(Default)]
pub struct Voices {
    pub slots: Vec<Option<Box<dyn Voice>>>,
    /// The parameters of the object the module's class controller made (the sputter's output), if it exists.
    pub object: Option<Vec<i32>>,
    /// Sounds that could not be loaded, so each is reported once.
    reported: Vec<u32>,
    /// A requested voice could not be decoded or started. The engine cannot promise a limiter replacement.
    pub(super) failed: bool,
}

/// A module's host for one update: its voices over a sampler.
pub struct PartHost<'a> {
    pub sampler: &'a mut dyn Sampler,
    pub bank: &'a str,
    pub voices: &'a mut Voices,
    /// What the map's makeup multiplies a voice by.
    pub makeup: f32,
}

impl PartHost<'_> {
    fn gain(&self, inputs: &PlayerInputs) -> f32 {
        inputs.gain() * self.makeup
    }
}

/// A sound that loops its loop points, else plays once.
fn rate(inputs: &PlayerInputs) -> f32 {
    inputs.pitch_ratio().max(MIN_RATE)
}

impl Host for PartHost<'_> {
    fn play(&mut self, player: usize, entry: &SampleEntry, inputs: &PlayerInputs) -> bool {
        if entry.kind != 0 {
            self.voices.failed = true;
            return false;
        }
        let data = match self.sampler.sound(self.bank, entry.index as usize) {
            Ok(data) => data,
            Err(e) => {
                self.voices.failed = true;
                if !self.voices.reported.contains(&entry.index) {
                    self.voices.reported.push(entry.index);
                    log::warn!("sample layer: {e}");
                }
                return false;
            }
        };
        if data.frames.is_empty() || data.sample_rate == 0 {
            self.voices.failed = true;
            return false;
        }
        let Some(voice) = self.sampler.start(data, self.gain(inputs), rate(inputs)) else {
            self.voices.failed = true;
            return false;
        };
        if self.voices.slots.len() <= player {
            self.voices.slots.resize_with(player + 1, || None);
        }
        self.voices.slots[player] = Some(voice);
        true
    }

    fn stop(&mut self, player: usize) {
        if let Some(Some(voice)) = self.voices.slots.get_mut(player) {
            voice.stop();
        }
        if let Some(slot) = self.voices.slots.get_mut(player) {
            *slot = None;
        }
    }

    fn pause(&mut self, player: usize) {
        if let Some(Some(voice)) = self.voices.slots.get_mut(player) {
            voice.pause();
        }
    }

    fn resume(&mut self, player: usize, inputs: &PlayerInputs) {
        let (gain, rate) = (self.gain(inputs), rate(inputs));
        let Some(Some(voice)) = self.voices.slots.get_mut(player) else { return };
        voice.set_gain(gain);
        voice.set_rate(rate);
        voice.resume();
    }

    fn update(&mut self, player: usize, inputs: &PlayerInputs) -> VoiceState {
        let (gain, rate) = (self.gain(inputs), rate(inputs));
        let Some(Some(voice)) = self.voices.slots.get_mut(player) else { return VoiceState::GONE };
        if inputs.changed(input::VOLUME) {
            voice.set_gain(gain);
        }
        if inputs.changed(input::PITCH) {
            voice.set_rate(rate);
        }
        let status = voice.status();
        if !status.playing {
            self.voices.slots[player] = None;
            return VoiceState::GONE;
        }
        VoiceState { playing: true, elapsed_ms: status.elapsed_ms, remaining_ms: status.remaining_ms }
    }

    fn class_call(&mut self, call: &ClassCall<'_>) -> i32 {
        self.voices.object = match call.released {
            true => None,
            false => Some(call.params.to_vec()),
        };
        i32::from(self.voices.object.is_some())
    }
}

#[cfg(test)]
mod tests;
