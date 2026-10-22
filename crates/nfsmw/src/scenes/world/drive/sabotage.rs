//! The engine sabotage of the scripted chase (`docs/specs/story-flow.md` section 2.4, `SabotageEngine`). The engine
//! takes no throttle while it is sabotaged; the rest of the car drives as it does. The original's sabotage level and its
//! damage are not modelled, so this cuts the throttle and the nitrous only.

use super::{Drive, DriveInput};

impl Drive {
    /// Sabotages the engine (`true`) or lets it run again (`false`).
    pub fn set_sabotaged(&mut self, sabotaged: bool) {
        self.sabotaged = sabotaged;
    }
}

impl DriveInput {
    /// The input of a sabotaged engine: no throttle and no nitrous, the steering and the brakes stay.
    pub(super) fn sabotaged(self, on: bool) -> Self {
        if !on {
            return self;
        }
        Self { throttle: 0.0, nos: false, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sabotaged_engine_takes_no_throttle_or_nitrous_and_keeps_the_brakes() {
        let held = DriveInput { throttle: 1.0, brake: 0.5, steer: 0.25, nos: true, ..DriveInput::default() };
        assert_eq!(held.sabotaged(false), held);
        let cut = held.sabotaged(true);
        assert_eq!(cut.throttle, 0.0);
        assert!(!cut.nos);
        assert_eq!((cut.brake, cut.steer), (0.5, 0.25));
    }
}
