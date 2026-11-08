//! What the rider asks of a bike for the game's input.

use blackbox_vehicle::InputState;
use blackbox_vehicle::bike::BikeInput;

/// `value` held to 0 to 1; 0 when it is not a number.
pub(super) fn unit(value: f32) -> f32 {
    match value.is_finite() {
        true => value.clamp(0.0, 1.0),
        false => 0.0,
    }
}

/// `value` held to -1 to 1; 0 when it is not a number.
pub(super) fn signed_unit(value: f32) -> f32 {
    match value.is_finite() {
        true => value.clamp(-1.0, 1.0),
        false => 0.0,
    }
}

/// What the rider asks of the bike for the game's input: the throttle and the stick as they are; the brake pedal pulls both
/// brakes and the handbrake only the rear one (the input layer has one brake axis). The gearbox is automatic: the shift
/// buttons, the gear keys and the clutch are not read.
pub(super) fn command(input: &InputState, stick_reach: f32) -> BikeInput {
    BikeInput {
        throttle: unit(input.throttle),
        front_brake: unit(input.brake),
        rear_brake: unit(input.brake).max(unit(input.handbrake)),
        // The stick reaches its end at the arcade steering's `stick_reach` of its travel (1 in the reference), at any speed.
        steer: signed_unit(input.steer / stick_reach.max(0.1)),
    }
}
