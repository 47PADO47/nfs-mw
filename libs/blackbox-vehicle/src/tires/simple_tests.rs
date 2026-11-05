use super::*;
use crate::ground::SurfaceGrip;

fn params(front: bool) -> TireParams {
    TireParams {
        radius: 0.33,
        grip_scale: 1.2,
        static_grip: 1.3,
        dynamic_grip: 1.0,
        brake_spec: 400.0,
        brake_lock_spec: 400.0,
        ebrake_spec: 2000.0,
        front,
    }
}

fn input(lat: f32, fwd: f32, load: f32) -> LoadedInput {
    LoadedInput {
        lat_vel: lat,
        fwd_vel: fwd,
        body_speed: fwd,
        load,
        dt: 1.0 / 60.0,
        quarter_mass: 400.0,
        surface: SurfaceGrip::DEFAULT,
    }
}

fn rolling(fwd: f32) -> Tire {
    Tire { av: fwd / 0.33, road_speed: fwd, ..Tire::default() }
}

#[test]
fn the_lateral_force_grows_with_the_slip_angle_to_twelve_degrees_and_stays_there() {
    // A grip scale low enough that the full lateral force stays inside the friction circle.
    let p = TireParams { grip_scale: 0.8, ..params(true) };
    let lateral = |degrees: f32| {
        let mut tire = rolling(30.0);
        let lat = 30.0 * degrees.to_radians().tan();
        tire.update_simple(&p, &input(lat, 30.0, 3000.0), 0.0)
    };
    let (three, six, twelve, twenty) = (lateral(3.0), lateral(6.0), lateral(12.0), lateral(20.0));
    assert!(three < 0.0, "the force opposes the slide: {three}");
    assert!((six / three - 2.0).abs() < 0.08, "linear below 12 degrees: {three} {six}");
    let full = 1.54 * 3000.0 * 0.8;
    assert!((twelve.abs() - full).abs() / full < 0.05, "1.54 * load * grip scale at 12 degrees: {twelve}");
    assert!((twenty.abs() - twelve.abs()).abs() / full < 0.08, "flat beyond: {twelve} {twenty}");
}

#[test]
fn the_friction_circle_caps_the_force_and_braking_widens_it() {
    let mut tire = rolling(30.0);
    tire.grip_boost = 2.0;
    let fy = tire.update_simple(&params(true), &input(15.0, 30.0, 3000.0), 0.5);
    assert!(fy.abs() <= 3000.0 * 1.3 + 1.0, "within load * static grip: {fy}");
    let mut braking = rolling(30.0);
    braking.grip_boost = 2.0;
    braking.brake = 1.0;
    let fy = braking.update_simple(&params(true), &input(15.0, 30.0, 3000.0), 0.5);
    assert!(fy.abs() > 3000.0 * 1.3 + 1.0, "the brake pedal widens the circle 2.5 times: {fy}");
}

#[test]
fn a_rolling_wheel_follows_the_road_and_drive_torque_pushes() {
    let mut tire = rolling(20.0);
    tire.drive_torque = 330.0;
    tire.update_simple(&params(false), &input(0.0, 20.0, 3000.0), 0.5);
    assert!((tire.longitudinal_force - 1000.0).abs() < 1.0, "torque / radius: {}", tire.longitudinal_force);
    assert!((tire.av - 20.0 / 0.33).abs() < 1e-3, "no slip: the wheel turns with the road");
    assert_eq!(tire.slip, 0.0);
}

#[test]
fn the_handbrake_locks_a_rear_wheel_into_a_skid_and_leaves_a_front_one_rolling() {
    let mut rear = rolling(20.0);
    rear.ebrake = 1.0;
    rear.update_simple(&params(false), &input(2.0, 20.0, 3000.0), 0.5);
    // The wheel stops at the end of the first locked step; the skid acts from the next.
    let fy = rear.update_simple(&params(false), &input(2.0, 20.0, 3000.0), 0.5);
    assert!(rear.brake_locked && rear.av == 0.0);
    assert!(rear.longitudinal_force < 0.0, "the skid slows the car: {}", rear.longitudinal_force);
    assert!(fy < 0.0, "and holds it sideways");
    let mut front = rolling(20.0);
    front.ebrake = 1.0;
    front.update_simple(&params(true), &input(2.0, 20.0, 3000.0), 0.5);
    assert!(!front.brake_locked);
}

#[test]
fn at_walking_pace_the_lateral_force_fades_with_the_sideways_speed() {
    let mut tire = rolling(0.5);
    let slow = tire.update_simple(&params(true), &input(0.2, 0.5, 3000.0), 0.5);
    let mut tire = rolling(0.5);
    let faster = tire.update_simple(&params(true), &input(0.4, 0.5, 3000.0), 0.5);
    assert!(slow.abs() < faster.abs());
}

#[test]
fn a_free_wheel_spins_up_with_its_torque_unless_the_handbrake_holds_a_rear_one() {
    let mut tire = Tire { drive_torque: 100.0, ..Tire::default() };
    tire.update_simple_free(&params(true), 0.5);
    assert!((tire.av - 5.0).abs() < 1e-5);
    let mut rear = Tire { av: 9.0, ebrake: 1.0, ..Tire::default() };
    rear.update_simple_free(&params(false), 0.5);
    assert_eq!(rear.av, 0.0);
}
