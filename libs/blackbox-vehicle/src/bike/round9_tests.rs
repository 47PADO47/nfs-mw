//! Round 9 (`docs/specs/ride6-feel-round9.md`): the load on a leaned tyre, the bike on its rear wheel, and a kerb under the front tyre.

use glam::Vec3;

use super::wheel_step::load_factor;
use super::{Bike, BikeInput, BikeSpec};
use crate::FIXED_STEP;
use crate::ground::{FlatGround, Ground, GroundHit, SurfaceGrip};

#[test]
fn a_leaned_bike_puts_less_load_on_the_tyre_for_the_same_spring_force_not_more() {
    // A fork axis raked by 26 degrees, on flat ground, bike upright and leaned 50 degrees to the right.
    let rake = 26.0_f32.to_radians();
    let upright_axis = Vec3::new(0.0, rake.cos(), -rake.sin());
    let upright = load_factor(Vec3::Y, Vec3::X, upright_axis);
    assert!((upright - 1.0 / rake.cos()).abs() < 1e-5, "{upright}");
    let lean = 50.0_f32.to_radians();
    let roll = glam::Mat3::from_rotation_z(-lean);
    let leaned = load_factor(Vec3::Y, roll * Vec3::X, roll * upright_axis);
    // The spring force divided by the cosine of the rake, times the cosine of the lean.
    assert!((leaned - lean.cos() / rake.cos()).abs() < 1e-4, "{leaned}");
    assert!(leaned < upright);
    // The cosine of the axis is held at one half.
    assert!((load_factor(Vec3::Y, Vec3::X, Vec3::new(0.0, 0.2, -0.98)) - 2.0).abs() < 1e-5);
}

/// A flat road with a raised step across it from `z = 6` on, seen by the vertical rays of the tyres.
struct Kerb(f32);

impl Ground for Kerb {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        if dir.y >= -1e-6 {
            return None;
        }
        let height = if origin.z >= 6.0 { self.0 } else { 0.0 };
        let distance = (height - origin.y) / dir.y;
        (distance >= 0.0 && distance <= max_distance).then_some(GroundHit {
            distance,
            normal: Vec3::Y,
            surface: SurfaceGrip::DEFAULT,
        })
    }
}

#[test]
fn a_front_tyre_that_meets_a_kerb_does_not_throw_the_bike() {
    for speed in [5.0_f32, 15.0, 30.0] {
        let mut bike = Bike::new(BikeSpec::example());
        assert!(bike.place_moving(&FlatGround::new(0.0), 0.0, 0.0, 2.0, 0.0, speed));
        let input = BikeInput { throttle: 0.3, ..Default::default() };
        for _ in 0..(3.0 / FIXED_STEP) as usize {
            bike.step(FIXED_STEP, &input, &Kerb(0.15));
        }
        assert!(!bike.fallen(), "a 15 cm kerb at {speed} m/s");
    }
}

#[test]
fn a_bike_on_its_rear_wheel_with_the_stick_held_stays_up() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 12.0));
    bike.set_weight(-1.0);
    let mut front_up = 0.0_f32;
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, steer: 0.3, ..Default::default() }, &ground);
        assert!(!bike.fallen(), "the bike rolled over at lean {}", bike.lean());
        if bike.wheel(0).on_ground {
            continue;
        }
        front_up += FIXED_STEP;
        assert!(bike.lean().abs() < 0.9, "lean {}", bike.lean());
    }
    // The example bike may or may not lift its front wheel; the check above is about the time it does.
    println!("front wheel up for {front_up:.2} s");
}
