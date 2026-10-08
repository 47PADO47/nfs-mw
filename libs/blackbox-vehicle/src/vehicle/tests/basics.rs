use super::*;
use crate::ground::NoGround;

/// Compression where `k c (1 + c p) = weight`.
fn static_compression(stiffness_lb_in: f32, progression: f32, weight: f32) -> f32 {
    let k = stiffness_lb_in * crate::math::LB_IN_TO_N_M;
    let mut c = weight / k;
    for _ in 0..30 {
        let f = k * c * (1.0 + c * progression) - weight;
        let df = k * (1.0 + 2.0 * c * progression);
        c -= f / df;
    }
    c
}

#[test]
fn free_fall_follows_gravity() {
    let mut spec = VehicleSpec::example();
    spec.aero.drag_coefficient = 0.0;
    spec.aero.aero_coefficient = 0.0;
    let mut v = Vehicle::new(spec);
    v.place(Vec3::new(0.0, 100.0, 0.0), identity());
    for _ in 0..60 {
        v.step(FIXED_STEP, &InputState::default(), &NoGround);
    }
    let t = 59.0 * FIXED_STEP;
    assert!((v.linear_velocity().y + 9.81 * t).abs() < 0.02, "vy = {}", v.linear_velocity().y);
    assert!((100.0 - v.position().y - 0.5 * 9.81).abs() < 0.2, "fallen {}", 100.0 - v.position().y);
    assert_eq!(v.wheels_on_ground(), 0);
}

#[test]
fn resting_on_flat_ground_settles_at_the_static_ride_height() {
    let v = parked();
    assert_eq!(v.wheels_on_ground(), 4);
    assert!(v.speed() < 0.02, "still moving at {}", v.speed());
    assert!(up(&v).y > 0.9999);

    let spec = v.spec();
    let w = spec.mass * 9.81;
    let front =
        static_compression(spec.chassis.spring_stiffness[0], spec.chassis.spring_progression[0], w * 0.55 / 2.0);
    let rear = static_compression(spec.chassis.spring_stiffness[1], spec.chassis.spring_progression[1], w * 0.45 / 2.0);
    let (cf, cr) = (v.wheel(0).compression, v.wheel(2).compression);
    assert!((cf - front).abs() < 0.1 * front, "front {cf} vs {front}");
    assert!((cr - rear).abs() < 0.1 * rear, "rear {cr} vs {rear}");
    assert!((v.wheel(0).compression - v.wheel(1).compression).abs() < 1e-3);
    // The wheel contact patches sit on the road.
    for i in 0..4 {
        assert!(v.wheel(i).position.y.abs() < 0.01, "patch {i} at {}", v.wheel(i).position.y);
    }
    let total_load: f32 = (0..4).map(|i| v.wheel(i).load).sum();
    assert!((total_load - w).abs() < 0.05 * w, "load {total_load} vs weight {w}");
}

#[test]
fn a_dropped_car_lands_and_settles() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place(Vec3::new(0.0, 1.2, 0.0), identity());
    let g = flat();
    let mut lowest = f32::MAX;
    for _ in 0..(5.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &InputState::default(), &g);
        lowest = lowest.min(v.position().y);
        assert!(v.position().is_finite());
    }
    assert!(v.speed() < 0.05, "speed {}", v.speed());
    assert_eq!(v.wheels_on_ground(), 4);
    // The body box never went through the road.
    assert!(lowest > 0.62, "lowest body height {lowest}");
}

#[test]
fn a_parked_car_stays_put() {
    let mut v = parked();
    let p = v.position();
    run(&mut v, &InputState::default(), 10.0);
    assert!((v.position() - p).length() < 0.01, "drifted {}", (v.position() - p).length());
}

#[test]
fn the_roll_axis_stays_level_at_rest() {
    let v = parked();
    let left = v.wheel(0).compression;
    let right = v.wheel(1).compression;
    assert!((left - right).abs() < 5e-4);
}
