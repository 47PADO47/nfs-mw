//! Nitrous: the loop while it burns and the purge when the tank runs dry. Spec §5.
//!
//! The loop plays at the mixer's own pitch (1.0). The mixer map sets the nitrous outputs from the inputs the game
//! publishes, which are the loop flag (input 1) and the purge flag (input 2) only (`car-sound-mixer.md` §2). The
//! nitrous pitch and low-pass inputs are never published, so they read 0 there too. The boost interpolator the
//! original runs on the nitrous (0.4 s rise, 1 s fall, `docs/specs/traffic-sound-events.md` §1.4) has no published
//! route, so it is not applied.

use super::{LoopId, SoundCommand, SoundRef};
use crate::input::CarInput;

#[derive(Debug, Clone, Default)]
pub(crate) struct NitrousFx {
    active: bool,
    empty: bool,
}

impl NitrousFx {
    pub fn tick(&mut self, input: &CarInput, out: &mut Vec<SoundCommand>) {
        if input.nos_active {
            out.push(SoundCommand::SetLoop { id: LoopId::Nitrous, sound: SoundRef::Nitrous, volume: 1.0, pitch: 1.0 });
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
