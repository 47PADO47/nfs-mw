//! The wheelie and stoppie control (`docs/specs/ride6-rider-help.md`): a pitch torque that holds the bike at the pitch the
//! rider's weight asks for, so a wheelie settles at its angle instead of flipping.

use super::curve::{factor, ramp};
use super::spec::{PitchParams, WheelieSpec};

/// What the assist is given.
#[derive(Clone, Copy, Debug)]
pub struct PitchInput {
    /// The rider's longitudinal weight, -1 (back) to 1 (forward).
    pub weight: f32,
    pub throttle: f32,
    pub front_brake: f32,
    /// The pitch (rad, positive nose up) and its rate (rad/s).
    pub pitch: f32,
    pub pitch_rate: f32,
    /// The speed along the bike (m/s) and the normalised lean (0 upright, 1 at the lean limit).
    pub speed: f32,
    pub normalised_lean: f32,
    pub front_on_ground: bool,
    pub rear_on_ground: bool,
    /// The height (m) of the wheel the assist raises over the ground.
    pub gap: f32,
    /// The pitch inertia of the bike without the rider (kg m^2): the help torque is scaled by it.
    pub inertia: f32,
}

/// The lever travel the rider asks the assist for, -1 (a wheelie at full) to 1 (a stoppie at full), after the dead zones.
pub fn target_position(spec: &WheelieSpec, weight: f32) -> f32 {
    let w = weight.clamp(-1.0, 1.0);
    match w >= 0.0 {
        true => ramp(w, [spec.stoppie.deadzone, 1.0]),
        false => -ramp(-w, [spec.wheelie.deadzone, 1.0]),
    }
}

/// The pitch torque (N m, positive raises the nose) that holds the pitch at the angle asked for; 0 when the assist is off.
pub fn torque(spec: &WheelieSpec, input: &PitchInput) -> f32 {
    let position = target_position(spec, input.weight);
    if position == 0.0 {
        return 0.0;
    }
    let (params, grounded, free_lever): (&PitchParams, bool, bool) = match position < 0.0 {
        // A wheelie: the rear tyre must be on the road and the brake off.
        true => (&spec.wheelie, input.rear_on_ground, input.front_brake < 0.1),
        // A stoppie: the front tyre on the road and the throttle off.
        false => (&spec.stoppie, input.front_on_ground, input.throttle < 0.1),
    };
    if !grounded || !free_lever || input.speed.abs() < params.min_speed {
        return 0.0;
    }
    // A wheelie (negative position) raises the nose (positive pitch), a stoppie lowers it.
    let target = -position * ramp(input.speed.abs(), params.speed_ramp) * params.max_angle;
    let current = input.pitch + input.pitch_rate * params.lookahead;
    let error = target - current;
    let acceleration = params.acceleration;
    let wanted_rate = error.signum() * (2.0 * acceleration * error.abs()).sqrt();
    let command =
        ((wanted_rate - input.pitch_rate) / params.acceleration_time.max(1e-3)).clamp(-acceleration, acceleration);
    let lean_help = 1.0 - ramp(input.normalised_lean.abs(), params.lean_help);
    let ground_help = 1.0 - ramp(input.gap.max(0.0), params.ground_range);
    let help = lean_help * ground_help * factor(&params.help_speed, input.speed.abs());
    input.inertia * command * help
}

/// A range that is `[0, 0]` is off.
fn ramp_on(x: f32, range: [f32; 2]) -> f32 {
    match range == [0.0, 0.0] {
        true => 0.0,
        false => ramp(x, range),
    }
}

/// The share of the throttle the rider help lets through when the front lifts: the lift angle (rad, positive nose up) and
/// the speed it lifts at (rad/s). While the rider asks for a wheelie the angle range of the forced wheelie applies.
pub fn throttle_factor(spec: &WheelieSpec, lift: f32, lift_speed: f32, wheelie_asked: bool) -> f32 {
    let range = match wheelie_asked {
        true => spec.forced_throttle_pitch,
        false => spec.throttle_pitch,
    };
    1.0 - ramp_on(lift, range).max(ramp_on(lift_speed, spec.throttle_pitch_speed))
}

/// The share of the front brake the rider help lets through when the rear lifts (the nose going down).
pub fn brake_factor(spec: &WheelieSpec, lift: f32, lift_speed: f32) -> f32 {
    1.0 - ramp_on(-lift, spec.brake_pitch).max(ramp_on(-lift_speed, spec.brake_pitch_speed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    fn input(weight: f32, pitch: f32, rate: f32) -> PitchInput {
        PitchInput {
            weight,
            throttle: 0.5,
            front_brake: 0.0,
            pitch,
            pitch_rate: rate,
            speed: 10.0,
            normalised_lean: 0.0,
            front_on_ground: true,
            rear_on_ground: true,
            gap: 0.0,
            inertia: 80.0,
        }
    }

    fn spec() -> WheelieSpec {
        BikeSpec::example().wheelie.unwrap()
    }

    #[test]
    fn the_dead_zones_swallow_a_small_lever_travel() {
        let s = spec();
        assert_eq!(target_position(&s, 0.0), 0.0);
        assert_eq!(target_position(&s, -0.2), 0.0);
        assert!(target_position(&s, -0.65) < 0.0 && target_position(&s, -0.65) > -1.0);
        assert_eq!(target_position(&s, 0.9), 0.0);
        assert_eq!(target_position(&s, 1.0), 1.0);
        assert_eq!(target_position(&s, -1.0), -1.0);
    }

    #[test]
    fn weight_back_raises_the_nose_and_weight_forward_lowers_it() {
        let s = spec();
        assert!(torque(&s, &input(-1.0, 0.0, 0.0)) > 0.0);
        let stoppie = PitchInput { throttle: 0.0, ..input(1.0, 0.0, 0.0) };
        assert!(torque(&s, &stoppie) < 0.0);
    }

    #[test]
    fn the_assist_holds_the_angle_it_was_asked_for() {
        let s = spec();
        let target = s.wheelie.max_angle;
        // At the target and at rest the torque is nothing; above it the nose is pulled down.
        assert!(torque(&s, &input(-1.0, target, 0.0)).abs() < 1.0);
        assert!(torque(&s, &input(-1.0, target + 0.2, 0.0)) < 0.0);
    }

    #[test]
    fn the_assist_needs_the_right_wheel_on_the_road_and_the_pedal_free() {
        let s = spec();
        assert_eq!(torque(&s, &PitchInput { rear_on_ground: false, ..input(-1.0, 0.0, 0.0) }), 0.0);
        assert_eq!(torque(&s, &PitchInput { front_brake: 0.5, ..input(-1.0, 0.0, 0.0) }), 0.0);
        assert_eq!(torque(&s, &PitchInput { throttle: 0.5, ..input(1.0, 0.0, 0.0) }), 0.0);
    }

    #[test]
    fn the_assist_fades_when_leaned_over_and_is_off_at_a_crawl() {
        let s = spec();
        let upright = torque(&s, &input(-1.0, 0.0, 0.0));
        assert!(torque(&s, &PitchInput { normalised_lean: 1.0, ..input(-1.0, 0.0, 0.0) }) < upright * 0.01);
        assert!(torque(&s, &PitchInput { speed: 0.0, ..input(-1.0, 0.0, 0.0) }).abs() < 1e-3);
    }

    #[test]
    fn the_torque_is_bounded_by_the_accelerations_the_tracker_allows() {
        let s = spec();
        let t = torque(&s, &input(-1.0, -1.0, -5.0));
        assert!(t.abs() <= 80.0 * s.wheelie.acceleration + 1e-3);
    }

    #[test]
    fn the_throttle_is_cut_as_the_front_lifts_and_the_front_brake_as_the_rear_does() {
        let s = spec();
        assert_eq!(throttle_factor(&s, 0.0, 0.0, false), 1.0);
        assert!(throttle_factor(&s, 0.04, 0.0, false) < 1.0);
        assert!(throttle_factor(&s, 0.2, 0.0, false) < 1e-6);
        // Asked for a wheelie, the throttle holds until the forced range.
        assert_eq!(throttle_factor(&s, 0.2, 0.0, true), 1.0);
        assert!(throttle_factor(&s, 0.8, 0.0, true) < 1e-6);
        assert!(throttle_factor(&s, 0.0, 0.5, false) < 1.0);
        assert_eq!(brake_factor(&s, 0.0, 0.0), 1.0);
        assert!(brake_factor(&s, -0.05, 0.0) < 1.0);
        assert!(brake_factor(&s, 0.3, 0.0) == 1.0);
    }

    #[test]
    fn a_zero_range_is_off() {
        let mut s = spec();
        s.throttle_pitch = [0.0, 0.0];
        s.throttle_pitch_speed = [0.0, 0.0];
        assert_eq!(throttle_factor(&s, 0.5, 1.0, false), 1.0);
    }

    #[test]
    fn the_help_fades_as_the_lifted_wheel_rises() {
        let s = spec();
        let low = torque(&s, &PitchInput { gap: 0.0, ..input(-1.0, 0.0, 0.0) });
        let high = torque(&s, &PitchInput { gap: 0.8, ..input(-1.0, 0.0, 0.0) });
        assert!(high > 0.0 && high < low * 0.3, "{low} {high}");
        assert!(torque(&s, &PitchInput { gap: 1.2, ..input(-1.0, 0.0, 0.0) }).abs() < 1e-3);
    }
}
