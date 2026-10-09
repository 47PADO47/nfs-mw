//! A car dragging a second, engineless body through a ball joint, on flat ground.

use super::*;
use crate::rigid_body::BallJoint;

/// Height of the hitch above the box centre of the example car, metres.
const HITCH_Y: f32 = -0.2;
/// Solver passes per step.
const PASSES: usize = 4;

/// A tractor and a trailer behind it, joined at the tractor's tail.
fn hitched() -> (Vehicle, Vehicle, BallJoint) {
    let mut tractor = Vehicle::new(VehicleSpec::example());
    let mut trailer = Vehicle::new(VehicleSpec::example());
    let tail = tractor.spec().dimension.z;
    let nose = trailer.spec().dimension.z;
    assert!(tractor.place_on_ground(&flat(), 0.0, 0.0, 5.0, 0.0));
    assert!(trailer.place_on_ground(&flat(), 0.0, -(tail + nose), 5.0, 0.0));
    // Nobody is driving the trailer: it must not hold itself still or reverse on its own.
    trailer.config.auto_brake = false;
    trailer.config.auto_reverse = false;
    let joint = BallJoint { anchor_a: Vec3::new(0.0, HITCH_Y, -tail), anchor_b: Vec3::new(0.0, HITCH_Y, nose) };
    (tractor, trailer, joint)
}

/// Steps both for `seconds`, returning the largest gap between the anchors.
fn drive(tractor: &mut Vehicle, trailer: &mut Vehicle, joint: &BallJoint, input: &InputState, seconds: f32) -> f32 {
    let g = flat();
    let mut worst = 0.0_f32;
    for _ in 0..(seconds / FIXED_STEP).round() as usize {
        tractor.step(FIXED_STEP, input, &g);
        trailer.step(FIXED_STEP, &InputState::default(), &g);
        joint.solve(tractor.body_mut(), trailer.body_mut(), FIXED_STEP, PASSES);
        worst = worst.max(joint.gap(tractor.body(), trailer.body()));
    }
    worst
}

#[test]
fn a_tractor_drags_its_trailer_straight_and_through_a_bend() {
    let (mut tractor, mut trailer, joint) = hitched();
    let throttle = InputState { throttle: 1.0, ..InputState::default() };
    let worst_straight = drive(&mut tractor, &mut trailer, &joint, &throttle, 5.0);
    assert!(tractor.forward_speed() > 5.0, "the tractor accelerates: {}", tractor.forward_speed());
    assert!(
        (tractor.forward_speed() - trailer.forward_speed()).abs() < 1.0,
        "the trailer keeps up: {} against {}",
        tractor.forward_speed(),
        trailer.forward_speed()
    );
    let bend = InputState { throttle: 0.5, steer: 0.6, ..InputState::default() };
    let worst_bend = drive(&mut tractor, &mut trailer, &joint, &bend, 6.0);
    assert!(worst_straight < 0.05 && worst_bend < 0.05, "gaps {worst_straight} and {worst_bend} m");
    let heading = |v: &Vehicle| (v.rotation().z_axis.x).atan2(v.rotation().z_axis.z);
    assert!(heading(&tractor).abs() > 0.3, "the tractor turned: {}", heading(&tractor));
    assert!(heading(&trailer).abs() > 0.1, "the trailer followed it round: {}", heading(&trailer));
    assert!(up(&tractor).y > 0.95 && up(&trailer).y > 0.95, "both stay upright");
    assert!(trailer.position().is_finite());
}
