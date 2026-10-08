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

/// Button presses that wait for a physics step. The physics runs at a fixed 60 Hz, so a frame at a higher rate
/// sometimes runs no step at all; a press that came in such a frame must not be lost (the original queues its
/// shift actions the same way).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Presses {
    shift_up: bool,
    shift_down: bool,
    reset: bool,
}

impl Presses {
    /// Remembers the presses of this frame's input.
    pub fn note(&mut self, input: &DriveInput) {
        self.shift_up |= input.shift_up;
        self.shift_down |= input.shift_down;
        self.reset |= input.reset;
    }

    /// `input` with the waiting presses in place of its own, which are now consumed.
    pub fn take_into(&mut self, input: DriveInput) -> DriveInput {
        let waiting = std::mem::take(self);
        DriveInput { shift_up: waiting.shift_up, shift_down: waiting.shift_down, reset: waiting.reset, ..input }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_waits_for_the_next_step() {
        let mut presses = Presses::default();
        let pressed = DriveInput { shift_up: true, reset: true, ..DriveInput::default() };
        // A frame with no physics step: the press is noted and nothing consumes it.
        presses.note(&pressed);
        // The next frame has no press of its own but a step.
        let later = DriveInput { throttle: 0.5, ..DriveInput::default() };
        presses.note(&later);
        let step = presses.take_into(later);
        assert!(step.shift_up && step.reset && !step.shift_down);
        assert_eq!(step.throttle, 0.5);
        assert!(!presses.take_into(later).shift_up, "a press is taken once");
    }

    #[test]
    fn shift_requests_fire_once_per_frame() {
        let input = DriveInput { throttle: 1.0, shift_up: true, ..DriveInput::default() };
        let later = input.held();
        assert!(!later.shift_up && later.throttle == 1.0);
    }
}
