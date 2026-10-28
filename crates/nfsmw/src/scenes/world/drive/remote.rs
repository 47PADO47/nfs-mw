//! A car driven from a network seat (ADR 0011, `docs/specs/multiplayer/world-host.md`): the server holds each player's
//! [`InputState`] for the tick and hands it to the car here in place of the devices' actions. The mapping between the
//! two input types is the one the car's physics step uses ([`DriveInput::to_vehicle`]), so a remote input drives the car
//! exactly as the same input from a device would.

use blackbox_vehicle::InputState;

use super::{CarPose, Drive, DriveInput};

/// A handbrake travel at or above this reads as the handbrake on: the car's handbrake is a button (`DriveInput`), and
/// the physics step sends it as travel 0 or 1.
pub const HANDBRAKE_ON: f32 = 0.5;

/// `x` clamped to `min..=max`, zero when not finite (a hostile or broken input).
fn axis(x: f32, min: f32, max: f32) -> f32 {
    match x.is_finite() {
        true => x.clamp(min, max),
        false => 0.0,
    }
}

impl DriveInput {
    /// What the vehicle's input shaping takes for this step (`blackbox_vehicle::input`). The handbrake button is full
    /// travel or none.
    pub fn to_vehicle(self) -> InputState {
        InputState {
            throttle: self.throttle,
            brake: self.brake,
            steer: self.steer,
            handbrake: f32::from(u8::from(self.handbrake)),
            nos: self.nos,
            shift_up: self.shift_up,
            shift_down: self.shift_down,
            gear_select: self.gear_select,
            clutch: self.clutch,
        }
    }

    /// The input a network seat sends, as the car takes it: the inverse of [`DriveInput::to_vehicle`]. Axes that are
    /// not finite read as zero and every axis is clamped to its range, as `from_actions` clamps a device; both shift
    /// edges at once become neither (the wire's rule, `nfsmw_net::input::InputCode::quantise`). The handbrake is on at
    /// [`HANDBRAKE_ON`] or more. Speedbreaker, reset and the scripted pop are not part of `InputState`: they stay off.
    pub fn from_vehicle(input: &InputState) -> Self {
        let both = input.shift_up && input.shift_down;
        Self {
            throttle: axis(input.throttle, 0.0, 1.0),
            brake: axis(input.brake, 0.0, 1.0),
            steer: axis(input.steer, -1.0, 1.0),
            // The rider's weight is not part of `InputState`: a network seat does not shift it.
            weight: 0.0,
            handbrake: axis(input.handbrake, 0.0, 1.0) >= HANDBRAKE_ON,
            nos: input.nos,
            speedbreaker: false,
            shift_up: input.shift_up && !both,
            shift_down: input.shift_down && !both,
            gear_select: input.gear_select,
            clutch: axis(input.clutch, 0.0, 1.0),
            reset: false,
            pop: false,
        }
    }
}

impl Drive {
    /// Drive from `input` instead of the devices from the next frame on, or go back to the devices with `None`. The
    /// input stays until it is replaced: a server sets it before every tick. Presses (shift edges, a gear key, reset)
    /// wait for the next physics step as a device's do.
    pub fn set_remote_input(&mut self, input: Option<DriveInput>) {
        self.remote_input = input;
    }

    /// The input a server holds for this seat, `None` for a car its own devices drive. A race seat reads its pedals
    /// from it (`players/race.rs`).
    pub fn remote_pedals(&self) -> Option<DriveInput> {
        self.remote_input
    }

    /// The network seat asked for a reset with its current input (the catch-up prompt reads it as a device's Reset).
    pub fn remote_reset(&self) -> bool {
        self.remote_input.is_some_and(|input| input.reset)
    }

    /// The pose after the last physics step, not interpolated: what a server replicates.
    pub fn stepped_pose(&self) -> CarPose {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_vehicle::drivetrain::{GEAR_FIRST, GEAR_REVERSE};

    /// Every value a device can give the car survives the trip to the vehicle input and back.
    #[test]
    fn device_input_round_trips_through_the_vehicle_input() {
        let inputs = [
            DriveInput::default(),
            DriveInput { throttle: 1.0, steer: -1.0, nos: true, ..DriveInput::default() },
            DriveInput { brake: 0.25, steer: 0.5, handbrake: true, clutch: 0.75, ..DriveInput::default() },
            DriveInput { shift_up: true, gear_select: Some(GEAR_FIRST + 3), ..DriveInput::default() },
            DriveInput { shift_down: true, gear_select: Some(GEAR_REVERSE), ..DriveInput::default() },
        ];
        for input in inputs {
            assert_eq!(DriveInput::from_vehicle(&input.to_vehicle()), input);
        }
    }

    #[test]
    fn the_handbrake_is_a_button_at_half_travel() {
        let travel = |handbrake| DriveInput::from_vehicle(&InputState { handbrake, ..InputState::default() }).handbrake;
        assert!(!travel(0.0) && !travel(7.0 / 15.0), "below half travel is off");
        assert!(travel(HANDBRAKE_ON) && travel(8.0 / 15.0) && travel(1.0));
        assert_eq!(DriveInput { handbrake: true, ..DriveInput::default() }.to_vehicle().handbrake, 1.0);
    }

    #[test]
    fn hostile_input_is_made_safe() {
        let wild = InputState {
            throttle: f32::NAN,
            brake: f32::INFINITY,
            steer: -1.0e30,
            handbrake: f32::NEG_INFINITY,
            clutch: 7.0,
            shift_up: true,
            shift_down: true,
            ..InputState::default()
        };
        let input = DriveInput::from_vehicle(&wild);
        assert_eq!((input.throttle, input.brake, input.steer, input.clutch), (0.0, 0.0, -1.0, 1.0));
        assert!(!input.handbrake && !input.shift_up && !input.shift_down, "both shift edges at once are neither");
    }

    #[test]
    fn one_shift_edge_passes_through() {
        let up = DriveInput::from_vehicle(&InputState { shift_up: true, ..InputState::default() });
        assert!(up.shift_up && !up.shift_down);
        let down = DriveInput::from_vehicle(&InputState { shift_down: true, ..InputState::default() });
        assert!(!down.shift_up && down.shift_down);
    }
}
