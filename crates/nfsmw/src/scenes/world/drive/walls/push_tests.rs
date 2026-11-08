//! What a wall's push may do to a body: never down, never far (`docs/specs/ride6-feel-round14.md`, section 3).

use blackbox_vehicle::{FlatGround, VehicleSpec};
use glam::Mat3;

use super::*;

/// A wall across the road at z = 2 that leans over it: its normal points back and a little down.
fn overhang(from: Vec3, to: Vec3) -> Option<Hit> {
    let crosses = from.z < 2.0 && to.z >= 2.0;
    let t = (2.0 - from.z) / (to.z - from.z);
    let normal = Vec3::new(0.0, -0.5, -0.866_025_4);
    crosses.then(|| Hit { point: from + (to - from) * t, normal, prop: None, info: None })
}

#[test]
fn a_push_never_goes_down_and_is_held_to_a_metre() {
    let leaning = Vec3::new(0.0, -0.5, -0.866_025_4);
    let push = push_out(leaning, 0.4);
    assert_eq!(push.y, 0.0, "{push}");
    assert!((push.z + 0.4 * 0.866_025_4).abs() < 1e-5, "the horizontal part is what the wall asked: {push}");
    assert!(push_out(Vec3::NEG_Z, 7.0).length() <= MAX_PUSH + 1e-6, "a deep contact does not throw the body");
    assert_eq!(push_out(Vec3::new(0.0, 0.5, -0.866_025_4), 0.4).y, 0.2, "a wall that leans away may lift it");
}

#[test]
fn a_car_in_a_wall_that_leans_over_the_road_is_not_pushed_into_the_road() {
    let mut car = Vehicle::new(VehicleSpec::example());
    assert!(car.place_on_ground(&FlatGround::new(0.0), 0.0, 0.0, 2.0, 0.0));
    let (before, previous) = (car.body().position, car.body().position);
    car.body_mut().position.z = 2.3;
    let walls = nfsmw_data::car::physics::WallSpec::default();
    let no_props = |_: Vec3, _: Mat3, _: Vec3| Vec::new();
    let mut scratch = Vec::new();
    let hit = resolve_into(&mut car, previous, &overhang, &no_props, &walls, &|_| 1.0, true, &mut scratch);
    assert!(hit.impulse > 0.0 || !scratch.is_empty(), "the wall was met");
    assert!(car.body().position.y >= before.y - 1e-4, "not pushed down: {} -> {}", before.y, car.body().position.y);
}
