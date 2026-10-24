//! Input ownership for a world-moment camera, without replacing the physical car.

use super::{Drive, DriveInput, Presses};

impl Drive {
    pub(in crate::scenes::world) fn set_cinematic_input(&mut self, input: Option<DriveInput>) {
        if self.cinematic_input.is_some() != input.is_some() {
            self.presses = Presses::default();
        }
        self.cinematic_input = input;
        if let Some(sim) = self.sim.as_mut() {
            sim.cinematic_control(input.is_some());
        }
        if input.is_some() {
            self.breaker.cancel();
        }
    }

    pub(in crate::scenes::world) fn physical_mass(&self) -> f32 {
        self.physics.spec.mass
    }
}

impl DriveInput {
    /// An active camera's AI fully owns the pedals and buttons; arrest and staging still have priority afterwards.
    pub(super) fn cinematic(self, ai: Option<Self>) -> Self {
        ai.unwrap_or(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moment_controls_replace_all_live_actions_and_release_to_current_inputs() {
        let player = DriveInput {
            throttle: 1.0,
            brake: 1.0,
            steer: -1.0,
            handbrake: true,
            nos: true,
            speedbreaker: true,
            shift_up: true,
            shift_down: true,
            gear_select: Some(7),
            clutch: 1.0,
            reset: true,
            pop: true,
        };
        let ai = DriveInput { throttle: 0.25, steer: 0.5, ..Default::default() };
        assert_eq!(player.cinematic(Some(ai)), ai);
        assert_eq!(player.cinematic(None), player);
        assert_eq!(
            player.cinematic(Some(ai)).during_arrest(true),
            DriveInput { brake: 1.0, handbrake: true, ..Default::default() }
        );
    }
}
