//! Arrest control override, independent of live player or scripted inputs.

use super::{Drive, DriveInput, Presses, PursuitCamera};

impl Drive {
    pub fn hold_for_arrest(&mut self, held: bool) {
        if self.arrested && !held {
            self.pursuit = PursuitCamera::default();
            self.presses = Presses::default();
        }
        self.arrested = held;
        if !held {
            return;
        }
        self.presses = Presses::default();
        self.breaker.cancel();
    }
}

impl DriveInput {
    pub(super) fn during_arrest(self, held: bool) -> Self {
        if !held {
            return self;
        }
        Self { brake: 1.0, handbrake: true, ..Self::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arrest_overrides_held_player_and_script_actions_until_release() {
        let held = DriveInput {
            throttle: 1.0,
            steer: 1.0,
            nos: true,
            speedbreaker: true,
            shift_up: true,
            shift_down: true,
            gear_select: Some(5),
            clutch: 1.0,
            reset: true,
            pop: true,
            ..Default::default()
        };
        assert_eq!(held.during_arrest(true), DriveInput { brake: 1.0, handbrake: true, ..Default::default() });
        assert_eq!(held.during_arrest(false), held);
    }
}
