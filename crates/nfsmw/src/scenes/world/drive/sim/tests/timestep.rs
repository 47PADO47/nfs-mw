//! Explicit adapter deltas, independently compared with the vehicle integrator.

use super::*;

fn state(vehicle: &Vehicle) -> Vec<u32> {
    let body = vehicle.body();
    body.position
        .to_array()
        .into_iter()
        .chain(body.orientation.to_array())
        .chain(body.linear_velocity.to_array())
        .chain(body.angular_velocity.to_array())
        .chain([vehicle.rpm(), vehicle.gear() as f32])
        .chain((0..4).flat_map(|i| {
            let wheel = vehicle.wheel(i);
            [wheel.angular_velocity, wheel.steer_angle, wheel.load, wheel.compression]
        }))
        .map(f32::to_bits)
        .collect()
}

#[test]
fn explicit_deltas_match_direct_vehicle_motion_spin_and_acceleration() {
    for dt in [FIXED_STEP, 1.0 / 144.0, 1.0 / 576.0] {
        let ground = FlatGround::new(5.0);
        let mut car = sim();
        assert!(car.place_moving(&ground, spawn_at(0.2), 25.0));
        let mut direct = car.vehicle.clone();
        let mut spin = [0.0; 4];
        for step in 0..120 {
            let steer = match step < 60 {
                true => 0.4,
                false => -0.2,
            };
            let input = DriveInput { throttle: 0.7, steer, ..Default::default() };
            let before = direct.body().linear_velocity;
            direct.step(dt, &input.to_vehicle(), &ground);
            car.step_with_delta(dt, &input, &ground, None);
            assert_eq!(state(&car.vehicle), state(&direct), "dt {dt}, step {step}");
            for (i, &physics) in PHYSICS_WHEEL.iter().enumerate() {
                spin[i] += direct.wheel(physics).angular_velocity * dt;
            }
            assert_eq!(car.spin, spin, "wheel animation must use the physical step interval");
            let acceleration = space::to_render(((direct.body().linear_velocity - before) / dt).to_array());
            assert_eq!(car.camera_acceleration(), acceleration);
        }
    }
}

#[test]
fn default_steps_preserve_the_explicit_sixty_hz_path() {
    let ground = FlatGround::new(5.0);
    let mut default = sim();
    let mut explicit = sim();
    for car in [&mut default, &mut explicit] {
        car.configure_ai();
        assert!(car.place_moving(&ground, spawn_at(0.0), 20.0));
    }
    for tick in 0..180 {
        let input = DriveInput { throttle: 0.5, brake: f32::from(u8::from(tick > 100)), ..Default::default() };
        default.step(&input, &ground, None);
        explicit.step_with_delta(FIXED_STEP, &input, &ground, None);
        assert_eq!(state(&default.vehicle), state(&explicit.vehicle));
        assert_eq!(default.spin, explicit.spin);
        assert_eq!(default.lamp_brake(), explicit.lamp_brake());
        assert_eq!(default.camera_acceleration(), explicit.camera_acceleration());
    }
}

#[test]
fn invalid_deltas_do_not_advance_motion_or_per_step_state() {
    let ground = FlatGround::new(5.0);
    let mut car = sim();
    assert!(car.place_moving(&ground, spawn_at(0.0), 20.0));
    let before = state(&car.vehicle);
    for dt in [0.0, -0.1, f32::NAN, f32::INFINITY] {
        car.step_with_delta(dt, &DriveInput { throttle: 1.0, brake: 1.0, ..Default::default() }, &ground, None);
        assert_eq!(state(&car.vehicle), before);
        assert_eq!(car.spin, [0.0; 4]);
        assert_eq!(car.step_delta, FIXED_STEP);
        assert_eq!(car.brake_lamp_age, 0.0);
    }
}

#[test]
fn npc_brake_admission_uses_elapsed_simulation_time_at_explicit_deltas() {
    let ground = FlatGround::new(0.0);
    for rate in [144, 576] {
        let mut car = sim();
        car.configure_traffic();
        let dt = 1.0 / rate as f32;
        for _ in 0..rate / 2 - 1 {
            car.step_with_delta(dt, &DriveInput { brake: 1.0, ..Default::default() }, &ground, None);
        }
        assert_eq!(car.lamp_brake(), 0.0, "rate {rate}");
        for _ in 0..3 {
            car.step_with_delta(dt, &DriveInput { brake: 1.0, ..Default::default() }, &ground, None);
        }
        assert_eq!(car.lamp_brake(), 1.0, "rate {rate}");
    }
}

#[test]
fn reset_grace_uses_elapsed_simulation_time_at_an_explicit_delta() {
    let ground = FlatGround::new(0.0);
    let mut car = sim();
    car.begin_reset_collision_grace();
    for _ in 0..144 * 2 {
        car.step_with_delta(1.0 / 144.0, &DriveInput::default(), &ground, None);
    }
    assert!(car.reset_collision_active(), "one-second gate plus grace has not elapsed");
    for _ in 0..180 {
        car.step_with_delta(1.0 / 144.0, &DriveInput::default(), &ground, None);
    }
    assert!(!car.reset_collision_active(), "grace expires after enough simulation time");
}

#[test]
fn a_shared_pair_impulse_changes_nitrous_admission_in_the_same_tick() {
    let mut cars = [sim(), sim()];
    for car in &mut cars {
        let mut spec = car.vehicle.spec().clone();
        spec.body.gravity = 0.0;
        spec.nos.nos_capacity = 120.0;
        spec.nos.torque_boost = 1.0;
        car.vehicle.set_spec(spec);
    }
    let half_length = cars[0].body().dimension().z;
    cars[0].body_mut().place(Vec3::NEG_Z * (half_length - 0.1), Quat::IDENTITY);
    cars[1].body_mut().place(Vec3::Z * (half_length - 0.1), Quat::IDENTITY);
    cars[0].body_mut().linear_velocity = Vec3::Z * 6.0;
    cars[1].body_mut().linear_velocity = Vec3::NEG_Z * 6.0;
    let input = DriveInput { throttle: 1.0, nos: true, ..Default::default() };
    let mut unobstructed = sim();
    unobstructed.vehicle = cars[0].vehicle.clone();
    unobstructed.step(&input, &NoGround, None);
    assert!(unobstructed.telemetry().nos_burning, "the incoming speed must admit nitrous");

    let starts = cars.each_mut().map(|car| {
        let start = car.begin_step_with_delta(FIXED_STEP, &input, false, None);
        car.begin_body(&start, None);
        start
    });
    let (first, second) = cars.split_at_mut(1);
    let hit = first[0].collide_with(&mut second[0]).expect("the approaching bodies must collide");
    assert!(hit.impulse > 0.0);
    assert!(cars[0].body().linear_velocity.length() < 4.4703, "contact must cross the nitrous speed gate");
    for (car, start) in cars.iter_mut().zip(&starts) {
        car.finish_body(start, &NoGround, None);
    }
    for (car, start) in cars.iter_mut().zip(&starts) {
        car.integration().run(start, &NoGround, None);
        car.end_step(start, &NoGround, None);
    }
    assert!(!cars[0].telemetry().nos_burning, "nitrous must read the post-contact speed this tick");
}

#[test]
fn invalid_deltas_do_not_advance_any_explicit_phase() {
    let ground = FlatGround::new(5.0);
    let mut car = sim();
    assert!(car.place_moving(&ground, spawn_at(0.0), 20.0));
    let before = state(&car.vehicle);
    for dt in [0.0, -0.1, f32::NAN, f32::INFINITY] {
        let start = car.begin_step_with_delta(dt, &DriveInput { throttle: 1.0, ..Default::default() }, false, None);
        car.begin_body(&start, None);
        car.finish_body(&start, &ground, None);
        car.integration().run(&start, &ground, None);
        car.end_step(&start, &ground, None);
        assert_eq!(state(&car.vehicle), before);
        assert_eq!(car.spin, [0.0; 4]);
        assert_eq!(car.step_delta, FIXED_STEP);
        assert_eq!(car.brake_lamp_age, 0.0);
    }
}
