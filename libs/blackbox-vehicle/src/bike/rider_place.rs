//! Where the rider is asked to sit (`docs/specs/ride6-rider-body-aero.md`, "Target place"): the rest place plus the shifts
//! the weight inputs, the lean and the speed ask for, and the air drag his posture adds.

use glam::Vec3;

use super::curve::{curve, ramp};
use super::rider_spec::RiderSpec;
use super::spec::ChassisSpec;

/// What decides the rider's place.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlaceInput {
    /// The longitudinal weight, -1 (back) to 1 (forward), and the lateral weight, -1 (left) to 1 (right).
    pub weight: f32,
    pub weight_lateral: f32,
    /// The target lean over the lean limit, -1 to 1, positive to the right.
    pub lean: f32,
    /// The speed along the bike (m/s).
    pub speed: f32,
}

fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// How far the lateral shift is asked for, -1 to 1: the lean and the lateral weight, each with its influence.
pub fn lateral_share(rider: &RiderSpec, input: &PlaceInput) -> f32 {
    (rider.lean_influence * input.lean + rider.weight_influence * input.weight_lateral).clamp(-1.0, 1.0)
}

/// The rider's asked-for place relative to his rest place on the bike (`rider_offset`), m: the base offset, the longitudinal
/// shift of the weight (the backward one for a weight below 0), and the lateral shift. At a standstill the stopped shifts
/// stand in for the moving ones, blended over `stopped_speed_range`.
pub fn place(chassis: &ChassisSpec, rider: &RiderSpec, input: &PlaceInput) -> Vec3 {
    let moving = ramp(input.speed.abs(), rider.stopped_speed_range);
    let weight = input.weight.clamp(-1.0, 1.0);
    let longitudinal = match weight <= 0.0 {
        true => lerp(rider.stopped_backward_offset, chassis.rider_backward_shift, moving) * -weight,
        false => lerp(rider.stopped_forward_offset, chassis.rider_forward_shift, moving) * weight,
    };
    let share = lateral_share(rider, input);
    let lateral = lerp(rider.stopped_lateral_offset, rider.lateral_offset, moving);
    // The sideways part follows the side of the lean; the vertical and the forward parts follow only its size.
    let sideways = Vec3::new(lateral.x * share, lateral.y * share.abs(), lateral.z * share.abs());
    rider.base_offset + longitudinal + sideways
}

/// The multiplier of the air drag from the rider's posture, relative to sitting upright: the drag curve at the tuck over the
/// curve at 0 (1 for a bike without a curve). The scale itself cancels out of the ratio.
pub fn drag_multiplier(rider: &RiderSpec, tuck: f32) -> f32 {
    if rider.drag_curve.is_empty() {
        return 1.0;
    }
    let upright = curve(&rider.drag_curve, 0.0);
    if upright.abs() < 1e-6 {
        return 1.0;
    }
    curve(&rider.drag_curve, tuck.clamp(0.0, 1.0)) / upright
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    fn setup() -> (ChassisSpec, RiderSpec) {
        let mut spec = BikeSpec::example();
        spec.chassis.rider_forward_shift = Vec3::new(0.0, 0.0, 0.1);
        spec.chassis.rider_backward_shift = Vec3::new(0.0, 0.0, -0.1);
        let rider = RiderSpec {
            lateral_offset: Vec3::new(-0.12, -0.05, 0.15),
            stopped_lateral_offset: Vec3::new(-0.25, 0.0, 0.0),
            stopped_forward_offset: Vec3::new(0.0, 0.0, 0.2),
            lean_influence: 1.0,
            weight_influence: 0.0,
            ..RiderSpec::default()
        };
        (spec.chassis, rider)
    }

    fn at(lean: f32, weight: f32, speed: f32) -> PlaceInput {
        PlaceInput { weight, lean, speed, ..PlaceInput::default() }
    }

    #[test]
    fn the_weight_moves_the_rider_along_the_bike_by_its_shifts() {
        let (chassis, rider) = setup();
        assert_eq!(place(&chassis, &rider, &at(0.0, 0.0, 20.0)), Vec3::ZERO);
        assert!((place(&chassis, &rider, &at(0.0, 1.0, 20.0)).z - 0.1).abs() < 1e-6);
        assert!((place(&chassis, &rider, &at(0.0, -0.5, 20.0)).z + 0.05).abs() < 1e-6);
    }

    #[test]
    fn the_lean_moves_the_rider_sideways_by_its_sign_and_down_and_forward_by_its_size() {
        let (chassis, rider) = setup();
        let right = place(&chassis, &rider, &at(0.5, 0.0, 20.0));
        let left = place(&chassis, &rider, &at(-0.5, 0.0, 20.0));
        assert!((right.x + 0.06).abs() < 1e-6 && (left.x - 0.06).abs() < 1e-6, "mirror image: {right:?} {left:?}");
        assert!(
            (right.y - left.y).abs() < 1e-7 && (right.z - left.z).abs() < 1e-7,
            "vertical and forward are symmetric"
        );
        assert!((right.y + 0.025).abs() < 1e-6 && (right.z - 0.075).abs() < 1e-6);
        // Beyond the limit nothing more is asked.
        assert_eq!(place(&chassis, &rider, &at(3.0, 0.0, 20.0)), place(&chassis, &rider, &at(1.0, 0.0, 20.0)));
    }

    #[test]
    fn the_stopped_shifts_stand_in_below_the_speed_range_and_fade_out_over_it() {
        let (chassis, mut rider) = setup();
        // The default range never uses them.
        assert!((place(&chassis, &rider, &at(1.0, 0.0, 0.0)).x + 0.12).abs() < 1e-6);
        rider.stopped_speed_range = [2.0, 6.0];
        assert!((place(&chassis, &rider, &at(1.0, 0.0, 0.0)).x + 0.25).abs() < 1e-6);
        assert!((place(&chassis, &rider, &at(1.0, 0.0, 4.0)).x + 0.185).abs() < 1e-6);
        assert!((place(&chassis, &rider, &at(1.0, 0.0, 9.0)).x + 0.12).abs() < 1e-6);
        assert!((place(&chassis, &rider, &at(0.0, 1.0, 0.0)).z - 0.2).abs() < 1e-6, "stopped forward shift");
        assert!((place(&chassis, &rider, &at(0.0, 1.0, 9.0)).z - 0.1).abs() < 1e-6);
    }

    #[test]
    fn the_lateral_weight_and_the_lean_add_up_within_their_influences() {
        let (chassis, mut rider) = setup();
        rider.weight_influence = 1.0;
        rider.lean_influence = 0.5;
        let input = PlaceInput { lean: 1.0, weight_lateral: 0.5, speed: 20.0, ..PlaceInput::default() };
        assert_eq!(lateral_share(&rider, &input), 1.0);
        assert!((place(&chassis, &rider, &input).x + 0.12).abs() < 1e-6);
        let opposite = PlaceInput { weight_lateral: -1.0, ..input };
        assert!((lateral_share(&rider, &opposite) + 0.5).abs() < 1e-6);
    }

    #[test]
    fn the_base_offset_is_always_added() {
        let (chassis, mut rider) = setup();
        rider.base_offset = Vec3::new(0.01, 0.02, 0.03);
        assert_eq!(place(&chassis, &rider, &at(0.0, 0.0, 20.0)), rider.base_offset);
    }

    #[test]
    fn tucking_in_lowers_the_drag_only_with_a_curve() {
        let mut rider = RiderSpec::default();
        assert_eq!(drag_multiplier(&rider, 1.0), 1.0);
        rider.drag_scale = 1.25;
        rider.drag_curve = vec![(0.0, 1.2), (1.0, 0.65)];
        assert!((drag_multiplier(&rider, 0.0) - 1.0).abs() < 1e-6);
        assert!((drag_multiplier(&rider, 1.0) - 0.65 / 1.2).abs() < 1e-6);
        assert!((drag_multiplier(&rider, 0.5) - 0.925 / 1.2).abs() < 1e-6);
        assert_eq!(drag_multiplier(&rider, 7.0), drag_multiplier(&rider, 1.0));
    }
}
