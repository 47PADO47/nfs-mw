//! Nitrous: the loop while it burns and the purge when the tank runs dry. Spec §5.

use super::{LoopId, SoundCommand, SoundRef};
use crate::engine::TICK_SECONDS;
use crate::input::CarInput;
use crate::math::slew;
use crate::tuning::NitrousTuning;

/// The pitch boost rises over 0.4 s and falls over 1 s.
const RISE_SECONDS: f32 = 0.4;
const FALL_SECONDS: f32 = 1.0;

#[derive(Debug, Clone)]
pub(crate) struct NitrousFx {
    tuning: NitrousTuning,
    active: bool,
    empty: bool,
    boost: f32,
}

impl NitrousFx {
    pub fn new(tuning: NitrousTuning) -> Self {
        Self { tuning, active: false, empty: false, boost: 0.0 }
    }

    pub fn tick(&mut self, input: &CarInput, out: &mut Vec<SoundCommand>) {
        let target = if input.nos_active { 1.0 } else { 0.0 };
        let step = TICK_SECONDS / if input.nos_active { RISE_SECONDS } else { FALL_SECONDS };
        self.boost = slew(self.boost, target, step);

        if input.nos_active {
            out.push(SoundCommand::SetLoop {
                id: LoopId::Nitrous,
                sound: SoundRef::Nitrous,
                volume: 1.0,
                pitch: 1.0 + self.tuning.pitch_boost * self.boost,
            });
        }
        if self.active && !input.nos_active {
            out.push(SoundCommand::StopLoop(LoopId::Nitrous));
        }
        self.active = input.nos_active;

        // The tank ran dry: one purge on the edge of the flag.
        if input.nos_empty && !self.empty {
            out.push(SoundCommand::Play { sound: SoundRef::Purge, volume: 1.0, pitch: 1.0 });
        }
        self.empty = input.nos_empty;
    }
}
