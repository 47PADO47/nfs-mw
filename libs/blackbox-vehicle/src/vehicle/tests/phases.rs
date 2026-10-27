use super::*;
use crate::ground::{Ground, GroundHit};
use crate::tires::TireIntegration;

struct EmptyGround;

impl Ground for EmptyGround {
    fn hit(&self, _: Vec3, _: Vec3, _: f32) -> Option<GroundHit> {
        None
    }
}

#[test]
fn contact_impulse_changes_nitrous_activation_in_the_same_response() {
    for dt in [FIXED_STEP, 1.0 / 144.0, 1.0 / 576.0] {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.body.position = Vec3::Y * 5.0;
        car.body.linear_velocity = Vec3::Z * 12.0;
        car.begin_body_step(dt);
        let mut untouched = car.clone();

        // A central stopping impulse represents the host's shared collision pass.
        // The engine must see the resulting speed, not the integration snapshot.
        let impulse = -car.body.linear_velocity * car.spec.mass;
        car.body.apply_impulse_at(impulse, car.body.world_cog());
        let input = InputState { throttle: 1.0, nos: true, ..Default::default() };
        for candidate in [&mut car, &mut untouched] {
            candidate.finish_body_step(dt, &EmptyGround);
            candidate.respond_step(dt, &input, &EmptyGround);
        }
        assert_eq!(car.powertrain.nos.boost, 1.0, "dt {dt}");
        assert_eq!(car.powertrain.nos.capacity, 1.0, "dt {dt}");
        assert!(untouched.powertrain.nos.boost > 1.0, "control must activate NOS, dt {dt}");
        assert!(untouched.powertrain.nos.capacity < 1.0, "dt {dt}");
    }
}

#[test]
fn body_drag_is_already_pending_when_late_creep_scales_lateral_force() {
    for integration in [TireIntegration::Reference, TireIntegration::Regularized] {
        for dt in [FIXED_STEP, 1.0 / 144.0, 1.0 / 576.0] {
            let mut spec = VehicleSpec::example();
            spec.body.gravity = 0.0;
            spec.body.drag = Vec3::X;
            spec.aero.drag_coefficient = 0.0;
            let mut car = Vehicle::new(spec);
            car.set_tire_integration(integration);
            car.body.position = Vec3::Y * 5.0;
            car.body.linear_velocity = Vec3::X * 0.25;
            car.begin_body_step(dt);
            car.finish_body_step(dt, &EmptyGround);
            let drag = car.body.pending_force().x;
            assert!(drag < 0.0);
            assert_eq!(car.body.linear_velocity, Vec3::X * 0.25, "drag integrates next tick");
            car.respond_step(dt, &InputState::default(), &EmptyGround);

            let retention = match integration {
                TireIntegration::Reference => 0.25,
                TireIntegration::Regularized => 0.75,
            };
            assert!((car.body.pending_force().x - drag * retention).abs() < 1e-6, "dt {dt}");
            assert_eq!(car.body.linear_velocity.x, 0.0625);
        }
    }
}

#[test]
fn standalone_step_matches_the_shared_phases_without_a_pair_contact() {
    for dt in [FIXED_STEP, 1.0 / 144.0, 1.0 / 576.0] {
        let mut standalone = parked();
        standalone.body.linear_velocity = Vec3::new(0.3, -0.1, 8.0);
        standalone.body.angular_velocity = Vec3::new(0.1, 0.2, 0.3);
        standalone.body.apply_force(Vec3::new(200.0, 100.0, 500.0));
        let mut phased = standalone.clone();
        let input = InputState { throttle: 0.4, steer: 0.3, ..Default::default() };
        standalone.step(dt, &input, &flat());
        phased.begin_body_step(dt);
        phased.finish_body_step(dt, &flat());
        phased.respond_step(dt, &input, &flat());
        assert_eq!(standalone.position(), phased.position());
        assert_eq!(standalone.orientation(), phased.orientation());
        assert_eq!(standalone.linear_velocity(), phased.linear_velocity());
        assert_eq!(standalone.angular_velocity(), phased.angular_velocity());
        assert_eq!(standalone.body.pending_force(), phased.body.pending_force());
        assert_eq!(standalone.body.pending_torque(), phased.body.pending_torque());
        assert_eq!(standalone.gear(), phased.gear());
        assert_eq!(standalone.powertrain.rpm(), phased.powertrain.rpm());
        for i in 0..4 {
            assert_eq!(standalone.wheel(i).position, phased.wheel(i).position);
            assert_eq!(standalone.wheel(i).simulated_angular_velocity, phased.wheel(i).simulated_angular_velocity);
        }
    }
}

#[test]
fn frozen_begin_clears_pending_forces_without_moving_or_responding() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body.position = Vec3::Y * 5.0;
    car.body.linear_velocity = Vec3::Z * 8.0;
    car.body.apply_force(Vec3::splat(200.0));
    car.body.apply_torque(Vec3::splat(100.0));
    car.body.state = crate::rigid_body::BodyState::Frozen;
    let position = car.position();
    let rpm = car.rpm();
    car.step(FIXED_STEP, &InputState { throttle: 1.0, ..Default::default() }, &EmptyGround);
    assert_eq!(car.position(), position);
    assert_eq!(car.linear_velocity(), Vec3::Z * 8.0);
    assert_eq!(car.body.pending_force(), Vec3::ZERO);
    assert_eq!(car.body.pending_torque(), Vec3::ZERO);
    assert_eq!(car.rpm(), rpm);
}

#[test]
fn a_pair_impulse_wakes_a_sleeping_car_before_its_zero_pedal_response() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body.position = Vec3::Y * 5.0;
    car.body.state = crate::rigid_body::BodyState::Asleep;
    car.begin_body_step(FIXED_STEP);
    car.body.apply_impulse_at(Vec3::Z * (car.spec.mass * 12.0), car.body.world_cog());
    car.finish_body_step(FIXED_STEP, &EmptyGround);
    car.respond_step(FIXED_STEP, &InputState { steer: 0.5, ..Default::default() }, &EmptyGround);
    assert_eq!(car.body.state, crate::rigid_body::BodyState::Awake);
    assert!(car.controls.steering > 0.0, "a pre-pair sleeping snapshot must not suppress response");
}

#[test]
fn sleeping_chassis_still_consumes_current_regular_brake_input() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body.state = crate::rigid_body::BodyState::Asleep;
    car.config.auto_reverse = false;
    car.config.auto_brake = false;
    for brake in [0.2, 0.0, 1.0, 0.0] {
        car.step(FIXED_STEP, &InputState { brake, ..Default::default() }, &EmptyGround);
        assert_eq!(car.controls.brake, brake);
        assert_eq!(car.body.state, crate::rigid_body::BodyState::Asleep);
    }
}

#[test]
fn reference_sleeping_live_car_runs_suspension_without_pedals() {
    for dt in [FIXED_STEP, 1.0 / 144.0, 1.0 / 576.0] {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.set_tire_integration(TireIntegration::Reference);
        car.body.position = Vec3::Y * 5.0;
        car.body.state = crate::rigid_body::BodyState::Asleep;
        let position = car.position();
        car.step(dt, &InputState::default(), &EmptyGround);
        assert_eq!(car.body.state, crate::rigid_body::BodyState::Awake);
        assert_eq!(car.position(), position, "late response does not integrate twice");
        let mut expected = car.body.clone();
        expected.begin_frame(dt);
        car.begin_body_step(dt);
        assert_eq!(car.position(), expected.position);
        assert_eq!(car.linear_velocity(), expected.linear_velocity);
    }
}

#[test]
fn reference_retired_sleeper_ticks_engine_without_suspension_wake() {
    for dt in [FIXED_STEP, 1.0 / 144.0, 1.0 / 576.0] {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.set_tire_integration(TireIntegration::Reference);
        car.body.position = Vec3::Y * 5.0;
        for _ in 0..120 {
            car.step(dt, &InputState { throttle: 1.0, ..Default::default() }, &EmptyGround);
        }
        car.set_retirement_sleep_delay(Some(2.0));
        car.disabled = true;
        car.body.state = crate::rigid_body::BodyState::Asleep;
        let rpm = car.rpm();
        let patches = car.patch_positions;
        let steering = car.wheel_angles;
        let position = car.position();
        car.step(dt, &InputState::default(), &EmptyGround);
        assert_ne!(car.rpm(), rpm, "the engine remains admitted");
        assert_eq!(car.body.state, crate::rigid_body::BodyState::Asleep);
        assert_eq!(car.position(), position);
        assert_eq!(car.patch_positions, patches);
        assert_eq!(car.wheel_angles, steering);
        assert_eq!(car.body.pending_force(), Vec3::ZERO);
        assert_eq!(car.body.pending_torque(), Vec3::ZERO);
    }
}
