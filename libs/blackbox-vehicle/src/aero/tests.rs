use super::*;

fn spec() -> AeroSpec {
    AeroSpec { drag_coefficient: 0.4, aero_coefficient: 0.005, aero_cg: 50.0, ..AeroSpec::default() }
}

fn input(v: Vec3) -> AeroInput {
    AeroInput {
        linear_velocity: v,
        rotation: Mat3::IDENTITY,
        gas: 1.0,
        ground_effect: 1.0,
        any_wheel_on_ground: true,
        tuning: 0.0,
    }
}

#[test]
fn drag_grows_with_the_square_of_speed_and_opposes_motion() {
    let a = forces(&spec(), &input(Vec3::new(0.0, 0.0, 20.0)), 1.3, -1.3);
    let b = forces(&spec(), &input(Vec3::new(0.0, 0.0, 40.0)), 1.3, -1.3);
    assert!(a.drag.z < 0.0);
    assert!((b.drag.z / a.drag.z - 4.0).abs() < 1e-4);
    assert!((a.drag.z + 0.4 * 20.0 * 20.0).abs() < 1e-3);
}

#[test]
fn lifting_off_the_throttle_doubles_drag() {
    let on = forces(&spec(), &input(Vec3::new(0.0, 0.0, 20.0)), 1.3, -1.3);
    let mut i = input(Vec3::new(0.0, 0.0, 20.0));
    i.gas = 0.0;
    let off = forces(&spec(), &i, 1.3, -1.3);
    assert!((off.drag.z / on.drag.z - 2.0).abs() < 1e-4);
    assert!((off.drag_drop - 0.1).abs() < 1e-6);
}

#[test]
fn downforce_is_linear_in_speed_and_pushes_down() {
    let a = forces(&spec(), &input(Vec3::new(0.0, 0.0, 20.0)), 1.3, -1.3);
    let b = forces(&spec(), &input(Vec3::new(0.0, 0.0, 40.0)), 1.3, -1.3);
    assert!(a.downforce.y < 0.0);
    assert!((b.downforce.y / a.downforce.y - 2.0).abs() < 1e-4);
    assert!((a.downforce.y + 20.0 * 2.0 * 0.005 * 1000.0).abs() < 1e-3);
    assert_eq!(a.downforce_z, Some(0.0));
}

#[test]
fn sideways_motion_loses_downforce_down_to_a_floor() {
    let fwd = forces(&spec(), &input(Vec3::new(0.0, 0.0, 20.0)), 1.3, -1.3);
    let side = forces(&spec(), &input(Vec3::new(20.0, 0.0, 0.0)), 1.3, -1.3);
    assert!((side.downforce.y / fwd.downforce.y - 0.4).abs() < 1e-4);
}

#[test]
fn zero_coefficients_disable_the_forces() {
    let f = forces(&AeroSpec::default(), &input(Vec3::new(0.0, 0.0, 30.0)), 1.3, -1.3);
    assert_eq!((f.drag, f.downforce), (Vec3::ZERO, Vec3::ZERO));
}

#[test]
fn tuning_scales_both_forces() {
    let base = forces(&spec(), &input(Vec3::new(0.0, 0.0, 20.0)), 1.3, -1.3);
    let mut i = input(Vec3::new(0.0, 0.0, 20.0));
    i.tuning = 1.0;
    let tuned = forces(&spec(), &i, 1.3, -1.3);
    assert!((tuned.drag.z / base.drag.z - 1.25).abs() < 1e-4);
    assert!((tuned.downforce.y / base.downforce.y - 1.25).abs() < 1e-4);
}

#[test]
fn an_airborne_car_keeps_only_a_small_share_of_its_downforce() {
    let mut i = input(Vec3::new(0.0, 0.0, 50.0));
    let grounded = forces(&spec(), &i, 1.3, -1.3);
    i.any_wheel_on_ground = false;
    i.ground_effect = 0.0;
    let flying = forces(&spec(), &i, 1.3, -1.3);
    assert!((flying.downforce.y / grounded.downforce.y - DEFAULT_AIRBORNE_SCALE).abs() < 1e-4);
}
