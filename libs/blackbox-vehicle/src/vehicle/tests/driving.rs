use super::*;
use crate::ground::{Ground, GroundHit, SlopedGround, SurfaceGrip};

fn throttle() -> InputState {
    InputState { throttle: 1.0, ..Default::default() }
}

#[test]
fn full_throttle_accelerates_and_shifts_up_through_the_gears() {
    let mut v = parked();
    let g = flat();
    let mut gears = vec![v.gear()];
    let mut reached_100kmh = None;
    for i in 0..(30.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &throttle(), &g);
        if *gears.last().unwrap() != v.gear() {
            gears.push(v.gear());
        }
        if reached_100kmh.is_none() && v.forward_speed() > 27.8 {
            reached_100kmh = Some(i as f32 * FIXED_STEP);
        }
        assert!(v.rpm() <= 7001.0, "rpm {}", v.rpm());
    }
    let t = reached_100kmh.expect("never reached 100 km/h");
    assert!(t > 3.0 && t < 9.0, "0-100 km/h in {t} s");
    assert!(v.forward_speed() > 45.0, "speed after 30 s: {}", v.forward_speed());
    // First gear is id 2; the car climbs one gear at a time and never hunts back down.
    assert!(gears.windows(2).all(|w| w[1] == w[0] + 1), "gears {gears:?}");
    assert!(*gears.last().unwrap() >= 6, "gears {gears:?}");
    // Straight line: no drift sideways, no yaw.
    assert!(v.position().x.abs() < 0.2, "x = {}", v.position().x);
    assert_eq!(v.wheels_on_ground(), 4);
}

#[test]
fn the_launch_spins_the_rear_wheels_but_not_the_front() {
    let mut v = parked();
    let g = flat();
    let mut max_rear_slip = 0.0_f32;
    for _ in 0..90 {
        v.step(FIXED_STEP, &throttle(), &g);
        max_rear_slip = max_rear_slip.max(v.wheel(2).slip);
        assert!(v.wheel(0).slip.abs() < 0.8, "front slip {}", v.wheel(0).slip);
    }
    assert!(max_rear_slip > 0.3, "rear slip only {max_rear_slip}");
}

#[test]
fn braking_stops_the_car_in_a_sensible_distance() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.config.auto_reverse = false;
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 30.0);
    let g = flat();
    let input = InputState { brake: 1.0, ..Default::default() };
    let start = v.position().z;
    let mut t = 0.0;
    while v.forward_speed() > 0.3 && t < 10.0 {
        v.step(FIXED_STEP, &input, &g);
        t += FIXED_STEP;
        assert!(v.forward_speed() > -0.5, "reversed to {}", v.forward_speed());
    }
    let distance = v.position().z - start;
    assert!(v.forward_speed() <= 0.3, "still doing {} after {t} s", v.forward_speed());
    // 30 m/s at about 0.9-1.1 g takes 40-50 m.
    assert!(distance > 30.0 && distance < 70.0, "stopped in {distance} m, {t} s");
    assert!(up(&v).y > 0.99, "nosedived: {}", up(&v).y);
    assert!(v.position().x.abs() < 0.5, "veered {} m", v.position().x);
}

#[test]
fn a_stationary_car_reverses_on_the_brake_pedal() {
    let mut v = parked();
    let input = InputState { brake: 1.0, ..Default::default() };
    run(&mut v, &input, 3.0);
    assert_eq!(v.gear(), crate::drivetrain::GEAR_REVERSE);
    assert!(v.forward_speed() < -1.5, "reversing at {}", v.forward_speed());
    assert!(v.position().x.abs() < 0.3);
    // Releasing the pedal returns to first gear and stops the car.
    run(&mut v, &InputState::default(), 5.0);
    assert_eq!(v.gear(), crate::drivetrain::GEAR_FIRST);
    assert!(v.forward_speed().abs() < 0.5, "speed {}", v.forward_speed());
}

#[test]
fn gentle_cornering_reaches_a_steady_state() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 20.0);
    let g = flat();
    let input = InputState { throttle: 0.6, steer: 0.2, ..Default::default() };
    let mut lateral = Vec::new();
    for i in 0..(14.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &input, &g);
        assert!(up(&v).y > 0.9, "rolled over at step {i}: up.y {}", up(&v).y);
        if i as f32 * FIXED_STEP > 8.0 {
            lateral.push(v.speed() * v.angular_velocity().y.abs());
        }
    }
    let mean = lateral.iter().sum::<f32>() / lateral.len() as f32;
    let spread = lateral.iter().map(|a| (a - mean).abs()).fold(0.0, f32::max);
    assert!(mean > 1.0 && mean < 12.0, "lateral acceleration {mean} m/s^2");
    assert!(spread < 0.25 * mean + 0.3, "not steady: mean {mean}, spread {spread}");
    // Turning right is a positive rotation about the up axis here.
    assert!(v.angular_velocity().y > 0.0, "yaw {}", v.angular_velocity().y);
    assert!(v.position().x > 5.0);
}

#[test]
fn hard_cornering_is_bounded_by_the_tires() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 30.0);
    let g = flat();
    let input = InputState { throttle: 0.5, steer: 1.0, ..Default::default() };
    let mut peak = 0.0_f32;
    for i in 0..(10.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &input, &g);
        assert!(up(&v).y > 0.5, "rolled over at step {i}");
        peak = peak.max(v.speed() * v.angular_velocity().y.abs());
        assert!(v.angular_velocity().length() < 20.0);
    }
    // Sticky road tires: more than 0.8 g but nowhere near 2.5 g.
    assert!(peak > 8.0 && peak < 24.0, "peak lateral acceleration {peak}");
}

#[test]
fn left_and_right_turns_mirror_each_other() {
    let g = flat();
    let mut left = Vehicle::new(VehicleSpec::example());
    let mut right = Vehicle::new(VehicleSpec::example());
    left.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 20.0);
    right.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 20.0);
    for _ in 0..(5.0 / FIXED_STEP) as usize {
        left.step(FIXED_STEP, &InputState { throttle: 0.3, steer: -0.4, ..Default::default() }, &g);
        right.step(FIXED_STEP, &InputState { throttle: 0.3, steer: 0.4, ..Default::default() }, &g);
    }
    assert!((left.position().x + right.position().x).abs() < 0.5, "{} {}", left.position().x, right.position().x);
    assert!((left.position().z - right.position().z).abs() < 0.5);
    assert!((left.speed() - right.speed()).abs() < 0.3);
}

#[test]
fn nitrous_adds_acceleration_and_drains_the_tank() {
    let g = flat();
    let mut plain = Vehicle::new(VehicleSpec::example());
    let mut boosted = Vehicle::new(VehicleSpec::example());
    plain.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 25.0);
    boosted.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 25.0);
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        plain.step(FIXED_STEP, &throttle(), &g);
        boosted.step(FIXED_STEP, &InputState { nos: true, ..throttle() }, &g);
    }
    assert!(
        boosted.forward_speed() > plain.forward_speed() + 1.0,
        "{} vs {}",
        boosted.forward_speed(),
        plain.forward_speed()
    );
    assert!(boosted.nos_level() < 0.6, "tank {}", boosted.nos_level());
    assert_eq!(plain.nos_level(), 1.0);
}

#[test]
fn the_handbrake_locks_the_rear_wheels() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 15.0);
    let input = InputState { handbrake: 1.0, ..Default::default() };
    run(&mut v, &input, 1.5);
    assert!(!v.wheel(0).locked);
    assert!(v.forward_speed() < 11.0, "speed {}", v.forward_speed());
    // The rear brakes bite harder than the free-rolling fronts: the rear wheels turn slower than the road.
    assert!(v.wheel(2).slip < -0.3 || v.wheel(2).locked, "rear slip {}", v.wheel(2).slip);
    run(&mut v, &input, 4.0);
    assert!(v.wheel(2).locked && v.wheel(3).locked);
}

#[test]
fn a_car_can_climb_a_slope_and_a_held_car_stays_on_it() {
    let slope = SlopedGround { height: 0.0, slope: 0.1, surface: SurfaceGrip::DEFAULT };
    let mut v = Vehicle::new(VehicleSpec::example());
    assert!(v.place_on_ground(&slope, 0.0, 0.0, 5.0, 0.0));
    let tilt = glam::Quat::from_rotation_x(-(0.1_f32).atan());
    let p = v.position();
    v.place(p, tilt);
    let hold = InputState { handbrake: 1.0, ..Default::default() };
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &hold, &slope);
    }
    assert_eq!(v.wheels_on_ground(), 4);
    assert!(v.speed() < 0.3, "the held car creeps at {}", v.speed());
    for _ in 0..(6.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &throttle(), &slope);
    }
    assert!(v.position().z > 20.0, "climbed only to z = {}", v.position().z);
    assert!(v.position().y > 2.0);
    assert!(up(&v).y > 0.9);
}

/// Ground that ends at z = 40.
struct Ledge;

impl Ground for Ledge {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        if origin.z > 40.0 {
            return None;
        }
        flat().hit(origin, dir, max_distance)
    }
}

#[test]
fn driving_off_a_ledge_is_survivable() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 30.0);
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &throttle(), &Ledge);
        assert!(v.position().is_finite() && v.linear_velocity().is_finite());
    }
    assert_eq!(v.wheels_on_ground(), 0);
    assert!(v.position().y < -10.0, "should be falling: y = {}", v.position().y);
}

#[test]
fn the_nitrous_burns_only_when_the_button_does_something() {
    // Held while standing: too slow, nothing burns, the sound must stay quiet.
    let mut v = parked();
    let nos = InputState { throttle: 1.0, nos: true, ..Default::default() };
    run(&mut v, &nos, 0.2);
    assert!(v.has_nos() && !v.nos_burning(), "burning at {} m/s", v.forward_speed());
    // Fast enough and on the throttle: it burns and the tank drains.
    run(&mut v, &throttle(), 3.0);
    let tank = v.nos_level();
    run(&mut v, &nos, 0.5);
    assert!(v.nos_burning() && v.nos_level() < tank);
    // Off the throttle it stops at once, the button still held.
    run(&mut v, &InputState { nos: true, ..Default::default() }, 0.2);
    assert!(!v.nos_burning());
}
