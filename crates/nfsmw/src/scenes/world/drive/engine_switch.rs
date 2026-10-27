//! The engine switch of the player's car (`bb.engine`, `crate::engine_switch`). An engine that a mod has stopped takes no
//! throttle and no nitrous, shows no revs and makes no sound; the steering, the brakes and everything else go on.

use super::{Drive, DriveInput};
use crate::engine_switch::{EngineAudio, EngineCommand};

impl Drive {
    /// A mod's request: switch the engine off, or start it.
    pub fn engine_apply(&mut self, command: EngineCommand) {
        self.engine.apply(command);
    }

    /// The engine and the pedals as the mods read them.
    pub fn engine_state(&self) -> blackbox_modding::host::EngineState {
        blackbox_modding::host::EngineState {
            phase: self.engine.phase(),
            free_roam: false,
            throttle: self.frame_input.throttle,
            brake: self.frame_input.brake,
            speed: self.speed(),
        }
    }

    /// What the car's sound needs of the switch this frame: whether the engine is silent, and the start sound once.
    pub(super) fn engine_audio(&mut self) -> EngineAudio {
        self.engine.audio()
    }
}

impl DriveInput {
    /// The input of a car whose engine is off: no throttle and no nitrous, the steering, the brakes and the gear
    /// requests stay.
    pub(super) fn engine_stopped(self, stopped: bool) -> Self {
        if !stopped {
            return self;
        }
        Self { throttle: 0.0, nos: false, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stopped_engine_takes_no_throttle_or_nitrous_and_keeps_the_rest() {
        let held =
            DriveInput { throttle: 1.0, brake: 0.5, steer: -0.25, nos: true, handbrake: true, ..Default::default() };
        assert_eq!(held.engine_stopped(false), held);
        let cut = held.engine_stopped(true);
        assert_eq!((cut.throttle, cut.nos), (0.0, false));
        assert_eq!((cut.brake, cut.steer, cut.handbrake), (0.5, -0.25, true));
    }
}
