//! Whole-vehicle behaviour tests on flat ground.

mod basics;
mod driving;
mod feel;
mod hitch;
mod manual;
mod robustness;

use glam::{Quat, Vec3};

use super::*;
use crate::FIXED_STEP;
use crate::ground::FlatGround;
use crate::input::InputState;

/// Body height of the example car with the springs at their unloaded length.
pub(super) const REST_Y: f32 = 0.74;

pub(super) fn flat() -> FlatGround {
    FlatGround::new(0.0)
}

/// An example car resting on the ground (springs at their unloaded length, then settled).
pub(super) fn parked() -> Vehicle {
    let mut v = Vehicle::new(VehicleSpec::example());
    assert!(v.place_on_ground(&flat(), 0.0, 0.0, 5.0, 0.0));
    run(&mut v, &InputState::default(), 3.0);
    v
}

/// Steps for `seconds` with a constant input.
pub(super) fn run(v: &mut Vehicle, input: &InputState, seconds: f32) {
    let g = flat();
    for _ in 0..(seconds / FIXED_STEP).round() as usize {
        v.step(FIXED_STEP, input, &g);
    }
}

pub(super) fn up(v: &Vehicle) -> Vec3 {
    v.orientation() * Vec3::Y
}

pub(super) fn identity() -> Quat {
    Quat::IDENTITY
}
