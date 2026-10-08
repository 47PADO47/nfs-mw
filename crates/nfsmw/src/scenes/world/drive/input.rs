//! What the driver asks of the car this step, from the player or from a script.

use crate::input::{Action, ActionState};

/// Pedals, steering and buttons for one physics step.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DriveInput {
    /// 0..1.
    pub throttle: f32,
    /// 0..1.
    pub brake: f32,
    /// -1 (left) ..1 (right).
    pub steer: f32,
    pub handbrake: bool,
    pub nos: bool,
    /// Shift requests; true for the one step after the button went down.
    pub shift_up: bool,
    pub shift_down: bool,
    /// The reset button went down: put the car back on the nearest road.
    pub reset: bool,
}

impl DriveInput {
    pub fn from_actions(actions: &ActionState) -> Self {
        Self {
            throttle: actions.value(Action::Throttle).clamp(0.0, 1.0),
            brake: actions.value(Action::Brake).clamp(0.0, 1.0),
            steer: actions.value(Action::Steer).clamp(-1.0, 1.0),
            handbrake: actions.pressed(Action::Handbrake),
            nos: actions.pressed(Action::Nos),
            shift_up: actions.just_pressed(Action::ShiftUp),
            shift_down: actions.just_pressed(Action::ShiftDown),
            reset: actions.just_pressed(Action::ResetCar),
        }
    }

    /// The same input with the one-shot shift requests cleared, for the second and later physics
    /// steps of a frame.
    pub fn held(self) -> Self {
        Self { shift_up: false, shift_down: false, reset: false, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_requests_fire_once_per_frame() {
        let input = DriveInput { throttle: 1.0, shift_up: true, ..DriveInput::default() };
        let later = input.held();
        assert!(!later.shift_up && later.throttle == 1.0);
    }
}
