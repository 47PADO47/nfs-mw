use super::*;
use crate::rigid_body::BodyState;
use crate::{FIXED_STEP, FlatGround, InputState, VehicleSpec};
use glam::Vec3;

fn candidate() -> Vehicle {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.set_retirement_sleep_delay(Some(2.0));
    car.wheels_on_ground = 4;
    car.body.linear_velocity = Vec3::ZERO;
    car.body.angular_velocity = Vec3::ZERO;
    car
}

#[test]
fn wreck_sleep_requires_two_supported_seconds_and_more_than_half_the_wheels() {
    let mut car = candidate();
    car.advance_retirement_support(1.0);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake);
    car.wheels_on_ground = 2;
    car.advance_retirement_support(0.5);
    assert_eq!(car.retirement.supported_time, 0.0);
    car.wheels_on_ground = 3;
    car.advance_retirement_support(1.5);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake);
    car.advance_retirement_support(0.5);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Asleep);
}

#[test]
fn wreck_sleep_keeps_authored_motion_threshold_and_current_object_contact_veto() {
    let mut car = candidate();
    let threshold = car.body.spec().sleep_velocity;
    car.body.linear_velocity = Vec3::Z * threshold;
    car.advance_retirement_support(2.0);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake, "the authored threshold is strict");
    car.body.linear_velocity = Vec3::ZERO;
    car.body.angular_velocity = Vec3::Y * (threshold * 2.0 / car.body.radius());
    car.advance_retirement_support(FIXED_STEP);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake, "angular motion still prevents sleep");
    car.body.angular_velocity = Vec3::ZERO;
    car.advance_retirement_support(FIXED_STEP);
    car.mark_object_contact();
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake);
    car.begin_retirement_frame();
    car.advance_retirement_support(FIXED_STEP);
    car.notify_collision(0.0);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Asleep, "a world-only notification is not the object-hit veto");
    car.mark_object_contact();
    assert_eq!(car.body.state, BodyState::Asleep, "a status mark alone does not wake a sleeping body");
}

#[test]
fn wreck_sleep_excludes_live_cars_and_pose_sync_does_not_advance_support_time() {
    let mut car = candidate();
    car.advance_retirement_support(1.5);
    for _ in 0..120 {
        car.finish_retirement_contacts();
    }
    assert_eq!(car.retirement.supported_time, 1.5);
    assert_eq!(car.body.state, BodyState::Awake);
    car.set_retirement_sleep_delay(None);
    car.advance_retirement_support(3.0);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake);
    assert_eq!(car.retirement.supported_time, 0.0);
}

#[test]
fn wreck_sleep_reaches_the_fixed_step_wheel_pipeline_persists_and_wakes_on_impulse() {
    let mut spec = VehicleSpec::example();
    // Explicit diagnostic threshold; installed cop fixtures separately verify their authored value.
    spec.body.sleep_velocity = 1.0;
    let mut car = Vehicle::new(spec);
    car.config.auto_brake = false;
    let ground = FlatGround::new(0.0);
    assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
    for _ in 0..180 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
        car.finish_retirement_contacts();
    }
    assert!(car.wheels_on_ground > 2);
    assert_eq!(car.body.state, BodyState::Awake, "a supported live car remains awake");
    car.set_retirement_sleep_delay(Some(2.0));
    for _ in 0..240 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
        car.finish_retirement_contacts();
    }
    assert_eq!(car.body.state, BodyState::Asleep);
    let position = car.body.position;
    for _ in 0..120 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
        car.finish_retirement_contacts();
    }
    assert_eq!(car.body.position, position);
    let impulse = Vec3::Z * car.body.mass() * 3.0;
    let centre = car.body.world_cog();
    car.body.apply_impulse_at(impulse, centre);
    assert_eq!(car.body.state, BodyState::Awake);
    car.step(FIXED_STEP, &InputState::default(), &ground);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake, "moving wrecks must remain physical");
}

#[test]
fn reference_body_end_reads_prior_support_timer_and_distinct_object_veto() {
    let mut car = candidate();
    car.set_tire_integration(crate::tires::TireIntegration::Reference);
    assert_eq!(car.reference_sleep_contacts(0), None);
    car.advance_retirement_support(1.0);
    assert_eq!(car.reference_sleep_contacts(0), None);
    car.advance_retirement_support(1.0);
    assert_eq!(car.reference_sleep_contacts(0), Some(4));
    car.notify_collision(0.0);
    assert_eq!(car.reference_sleep_contacts(0), Some(4));
    car.mark_object_contact();
    assert_eq!(car.reference_sleep_contacts(0), None);
    car.begin_retirement_frame();
    car.set_retirement_sleep_delay(None);
    assert_eq!(car.reference_sleep_contacts(0), None);
    car.wheels_on_ground = 0;
    assert_eq!(car.reference_sleep_contacts(3), Some(3));
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake, "late pose sync cannot put Reference bodies to sleep");
}

#[test]
fn reference_retirement_crossing_waits_for_the_next_body_end() {
    let mut car = candidate();
    car.set_tire_integration(crate::tires::TireIntegration::Reference);
    car.body.position = Vec3::Y * 5.0;
    car.advance_retirement_support(1.5);
    car.finish_body_step(FIXED_STEP, &FlatGround::new(0.0));
    assert_eq!(car.body.state, BodyState::Awake);
    car.advance_retirement_support(0.5);
    car.finish_retirement_contacts();
    assert_eq!(car.body.state, BodyState::Awake, "the later callback cannot sleep the body this tick");
    car.finish_body_step(FIXED_STEP, &FlatGround::new(0.0));
    assert_eq!(car.body.state, BodyState::Asleep);
}
