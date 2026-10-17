//! Authored global common events. Spec: `common-audio-events.md`.

#[cfg(test)]
mod tests;

use blackbox_mixmap::{InputKey, Mixer, ObjectRef};

use super::aems::{DelayedSampler, EffectModule};
use super::{Audio, Group};

const BANK: &str = "IG_GLOBAL/FX_Misc_MB.abk";
const OBJECT: ObjectRef = ObjectRef { state: 0, instance: 0, object: 4 };
const TICK: f64 = 1.0 / 60.0;
const MAX_TICKS: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonEvent {
    VisualPulse,
    SpeedbreakerBegin,
    SpeedbreakerEnd,
}

impl CommonEvent {
    fn slot(self) -> usize {
        match self {
            Self::VisualPulse => 0,
            Self::SpeedbreakerBegin => 1,
            Self::SpeedbreakerEnd => 2,
        }
    }

    fn input(self) -> u8 {
        [1, 3, 4][self.slot()]
    }

    fn words(self, volume: i32) -> [i32; 6] {
        [[0, 2, 1][self.slot()], volume.clamp(0, 32767), 0, 0, 0, 0]
    }
}

struct Running {
    module: EffectModule,
    event: CommonEvent,
}

/// Persistent main-state mixer and independent authored one-shot lifetimes.
#[derive(Default)]
pub(in crate::audio) struct CommonPlayback {
    mixer: Option<Mixer>,
    active: [Option<Running>; 3],
    pulses: [bool; 3],
    debt: f64,
    seed: u32,
    unavailable: bool,
}

impl Audio {
    pub fn update_common(&mut self, events: Vec<CommonEvent>, dt: f32) {
        let mut playback = std::mem::take(&mut self.common);
        for event in events {
            if self.trace.enabled() {
                self.trace.note("common_event", &format!("{event:?}"));
            }
            playback.trigger(self, event);
        }
        playback.update(self, dt);
        self.common = playback;
    }
}

impl CommonPlayback {
    pub fn trigger(&mut self, audio: &mut Audio, event: CommonEvent) -> bool {
        if self.unavailable {
            return false;
        }
        if self.mixer.is_none() {
            let map = match crate::audio::mixer::mix_map(&audio.dir) {
                Ok(map) => map,
                Err(error) => {
                    log::warn!("common sound mixer unavailable: {error:#}");
                    self.unavailable = true;
                    return false;
                }
            };
            let mut counts = [0; 13];
            counts[0] = 1;
            let mut mixer = Mixer::new(map, &counts);
            mixer.attach(OBJECT, true);
            for index in 0..4 {
                mixer.set_input(InputKey::controller(0, 0, 0, index), 0);
            }
            self.mixer = Some(mixer);
        }
        let slot = event.slot();
        self.pulses[slot] = true;
        if event == CommonEvent::VisualPulse && self.active[slot].is_some() {
            return true;
        }
        self.active[slot] = None;
        self.seed = self.seed.wrapping_add(1);
        match EffectModule::load(audio, BANK, "FX_UVES", self.seed) {
            Ok(module) => {
                self.active[slot] = Some(Running { module, event });
                true
            }
            Err(error) => {
                log::warn!("FX_UVES unavailable: {error}");
                false
            }
        }
    }

    pub fn update(&mut self, audio: &mut Audio, dt: f32) {
        let Some(mixer) = &mut self.mixer else {
            return;
        };
        if dt.is_finite() && dt > 0.0 {
            self.debt += f64::from(dt);
        }
        let mut ticks = 0;
        while self.debt + 1e-9 >= TICK && ticks < MAX_TICKS {
            self.debt = (self.debt - TICK).max(0.0);
            for event in [CommonEvent::VisualPulse, CommonEvent::SpeedbreakerBegin, CommonEvent::SpeedbreakerEnd] {
                mixer.set_input(InputKey::object(0, 0, 4, event.input()), i32::from(self.pulses[event.slot()]) * 32767);
            }
            mixer.process(TICK as f32);
            self.pulses = [false; 3];
            for (slot, running) in self.active.iter_mut().enumerate() {
                let Some(current) = running else {
                    continue;
                };
                let gain = (mixer.volume(OBJECT, slot + 2).unwrap_or(0.0) * 32767.0) as i32;
                let words = current.event.words(gain);
                let healthy = current.module.update(
                    &mut DelayedSampler(audio, Group::Sfx, ticks as f32 / 60.0),
                    TICK as f32,
                    &words,
                );
                if !healthy || current.module.finished() {
                    *running = None;
                }
            }
            ticks += 1;
        }
    }
}
