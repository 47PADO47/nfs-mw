//! What the AEMS modules ask of the sound output: bank sounds played as voices.

use crate::audio::trace::{Record, Trace};
use blackbox_aems::{ClassCall, Host, PlayerInputs, SampleEntry, VoiceState, input};
use kira::sound::static_sound::StaticSoundData;

pub(super) struct ModuleTrace {
    sink: Trace,
    instance: u64,
    bank: String,
    class: String,
    logical_ms: f64,
}

impl ModuleTrace {
    pub fn instance(&self) -> u64 {
        self.instance
    }
    pub fn new(sink: Trace, bank: &str, class: &str) -> Self {
        let value =
            Self { instance: sink.instance(), logical_ms: sink.now_ms(), sink, bank: bank.into(), class: class.into() };
        value.event("module_create", None, None, None, "", &[]);
        value
    }
    fn event(
        &self,
        event: &str,
        player: Option<usize>,
        inputs: Option<&PlayerInputs>,
        success: Option<bool>,
        detail: &str,
        params: &[i32],
    ) {
        self.sink.record(Record {
            event,
            instance: self.instance,
            bank: &self.bank,
            class: &self.class,
            logical_ms: self.logical_ms,
            params,
            player,
            inputs,
            success,
            detail,
            ..Default::default()
        });
    }
}

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
    fn set_channels(&mut self, _channels: u16) {}
    fn set_effects(&mut self, _effects: VoiceEffects) {}
    fn set_gain(&mut self, gain: f32);
    /// The playback rate, 1 = natural.
    fn set_rate(&mut self, rate: f32);
    fn pause(&mut self);
    fn resume(&mut self);
    fn stop(&mut self);
    fn status(&self) -> VoiceStatus;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceEffects {
    pub azimuth: u16,
    pub low_pass: f32,
    pub high_pass: f32,
    pub dry: f32,
    pub wet: f32,
}

impl VoiceEffects {
    fn read(inputs: &PlayerInputs) -> Self {
        let gain = |kind| inputs.get(kind).map_or(1.0, |v| v.clamp(0, 32767) as f32 / 32767.0);
        Self {
            azimuth: inputs.get(input::AZIMUTH).unwrap_or(0) as u16,
            low_pass: inputs.get(input::LOW_PASS).unwrap_or(25_000).clamp(0, 25_000) as f32,
            high_pass: inputs.get(input::HIGH_PASS).unwrap_or(0).clamp(0, 25_000) as f32,
            dry: gain(input::DRY),
            wet: inputs.get(input::REVERB).unwrap_or(0).clamp(0, 32767) as f32 / 32767.0,
        }
    }
}

/// The sound output the modules play through.
pub trait Sampler {
    fn channels(&self, _bank: &str, _index: usize) -> u16 {
        1
    }
    /// Bank sound `index` (counted as in the bank's `BNKl`) of the bank at `bank`, relative to `SOUND/`.
    fn sound(&mut self, bank: &str, index: usize) -> Result<StaticSoundData, String>;
    /// Starts `data` at `gain` (linear) and playback rate `rate`.
    fn start(&mut self, data: StaticSoundData, gain: f32, rate: f32) -> Option<Box<dyn Voice>>;
    fn start_with(
        &mut self,
        data: StaticSoundData,
        gain: f32,
        rate: f32,
        channels: u16,
        effects: VoiceEffects,
    ) -> Option<Box<dyn Voice>> {
        let mut voice = self.start(data, gain, rate)?;
        voice.set_channels(channels);
        voice.set_effects(effects);
        Some(voice)
    }
}

/// The slowest playback the voices are asked for (an input of 0 would stop time).
const MIN_RATE: f32 = 0.01;

/// The voices and the object of one running module.
#[derive(Default)]
pub struct Voices {
    pub(super) trace: Option<ModuleTrace>,
    pub slots: Vec<Option<Box<dyn Voice>>>,
    /// The parameters of the object the module's class controller made (the sputter's output), if it exists.
    pub object: Option<Vec<i32>>,
    /// Sounds that could not be loaded, so each is reported once.
    reported: Vec<u32>,
    /// A requested voice could not be decoded or started. The engine cannot promise a limiter replacement.
    pub(super) failed: bool,
    /// Voices the module has started since [`Voices::take_started`] (for the sputter, its pops).
    started: u32,
}

impl Voices {
    pub(super) fn trace_tick(&mut self, params: &[i32]) {
        if let Some(trace) = &mut self.trace {
            trace.logical_ms += 1000.0 / 60.0;
            trace.event("module_tick", None, None, None, "", params);
        }
    }
    pub(super) fn trace_event(
        &self,
        event: &str,
        player: Option<usize>,
        inputs: Option<&PlayerInputs>,
        success: Option<bool>,
        detail: &str,
    ) {
        if let Some(trace) = &self.trace {
            trace.event(event, player, inputs, success, detail, &[]);
        }
    }
    #[cfg(test)]
    pub(super) fn counts(&self) -> (u32, usize) {
        (self.started, self.slots.iter().flatten().filter(|voice| voice.status().playing).count())
    }
    /// The voices started since the last call.
    pub fn take_started(&mut self) -> u32 {
        std::mem::take(&mut self.started)
    }
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

    /// Creation uses seven-bit play options; changed updates retain Q15 precision.
    fn initial_gain(&self, inputs: &PlayerInputs) -> f32 {
        let word = inputs.get(input::VOLUME).unwrap_or(32767).clamp(0, 32767);
        creation_gain(word) * self.makeup
    }
}

fn creation_gain(word: i32) -> f32 {
    ((word.clamp(0, 32767) >> 8) as f32) / 127.0
}

/// A sound that loops its loop points, else plays once.
fn rate(inputs: &PlayerInputs) -> f32 {
    inputs.pitch_ratio().max(MIN_RATE)
}

impl Host for PartHost<'_> {
    fn play(&mut self, player: usize, entry: &SampleEntry, inputs: &PlayerInputs) -> bool {
        // Bank indexes start at one. An authored zero entry intentionally has no recording.
        if entry.kind == 0 && entry.index == 0 {
            self.voices.trace_event("voice_silent", Some(player), Some(inputs), Some(false), "authored zero recording");
            return false;
        }
        if entry.kind != 0 {
            self.voices.trace_event(
                "voice_failure",
                Some(player),
                Some(inputs),
                Some(false),
                "unsupported recording kind",
            );
            self.voices.failed = true;
            return false;
        }
        let data = match self.sampler.sound(self.bank, entry.index as usize) {
            Ok(data) => data,
            Err(e) => {
                self.voices.trace_event("voice_failure", Some(player), Some(inputs), Some(false), &e);
                self.voices.failed = true;
                if !self.voices.reported.contains(&entry.index) {
                    self.voices.reported.push(entry.index);
                    log::warn!("sample layer: {e}");
                }
                return false;
            }
        };
        if data.frames.is_empty() || data.sample_rate == 0 {
            self.voices.trace_event(
                "voice_failure",
                Some(player),
                Some(inputs),
                Some(false),
                "empty recording or zero sample rate",
            );
            self.voices.failed = true;
            return false;
        }
        let channels = self.sampler.channels(self.bank, entry.index as usize);
        let Some(mut voice) = self.sampler.start_with(
            data,
            self.initial_gain(inputs),
            rate(inputs),
            channels,
            VoiceEffects::read(inputs),
        ) else {
            self.voices.trace_event("voice_failure", Some(player), Some(inputs), Some(false), "backend start failed");
            self.voices.failed = true;
            return false;
        };
        voice.set_channels(self.sampler.channels(self.bank, entry.index as usize));
        voice.set_effects(VoiceEffects::read(inputs));
        if self.voices.slots.len() <= player {
            self.voices.slots.resize_with(player + 1, || None);
        }
        self.voices.slots[player] = Some(voice);
        self.voices.started = self.voices.started.saturating_add(1);
        self.voices.trace_event(
            "voice_start",
            Some(player),
            Some(inputs),
            Some(true),
            &format!(
                "sample={} creation_gain={} makeup={} rate={}",
                entry.index,
                self.initial_gain(inputs),
                self.makeup,
                rate(inputs)
            ),
        );
        true
    }

    fn stop(&mut self, player: usize) {
        self.voices.trace_event("voice_stop", Some(player), None, None, "");
        if let Some(Some(voice)) = self.voices.slots.get_mut(player) {
            voice.stop();
        }
        if let Some(slot) = self.voices.slots.get_mut(player) {
            *slot = None;
        }
    }

    fn pause(&mut self, player: usize) {
        self.voices.trace_event("voice_pause", Some(player), None, None, "");
        if let Some(Some(voice)) = self.voices.slots.get_mut(player) {
            voice.pause();
        }
    }

    fn resume(&mut self, player: usize, inputs: &PlayerInputs) {
        self.voices.trace_event("voice_resume", Some(player), Some(inputs), None, "");
        let (gain, rate) = (self.gain(inputs), rate(inputs));
        let Some(Some(voice)) = self.voices.slots.get_mut(player) else { return };
        voice.set_gain(gain);
        voice.set_rate(rate);
        voice.set_effects(VoiceEffects::read(inputs));
        voice.resume();
    }

    fn update(&mut self, player: usize, inputs: &PlayerInputs) -> VoiceState {
        self.voices.trace_event("voice_update", Some(player), Some(inputs), None, "");
        let (gain, rate) = (self.gain(inputs), rate(inputs));
        let Some(Some(voice)) = self.voices.slots.get_mut(player) else { return VoiceState::GONE };
        if inputs.changed(input::VOLUME) {
            voice.set_gain(gain);
        }
        if inputs.changed(input::PITCH) {
            voice.set_rate(rate);
        }
        voice.set_effects(VoiceEffects::read(inputs));
        let status = voice.status();
        if !status.playing {
            self.voices.trace_event("voice_end", Some(player), Some(inputs), Some(true), "backend completed");
            self.voices.slots[player] = None;
            return VoiceState::GONE;
        }
        VoiceState { playing: true, elapsed_ms: status.elapsed_ms, remaining_ms: status.remaining_ms }
    }

    fn class_call(&mut self, call: &ClassCall<'_>) -> i32 {
        if let Some(trace) = &self.voices.trace {
            trace.event(
                "class_object",
                Some(call.controller),
                None,
                None,
                if call.released { "released" } else { "created or updated" },
                call.params,
            );
        }
        self.voices.object = match call.released {
            true => None,
            false => Some(call.params.to_vec()),
        };
        i32::from(self.voices.object.is_some())
    }
}

#[cfg(test)]
mod tests;
