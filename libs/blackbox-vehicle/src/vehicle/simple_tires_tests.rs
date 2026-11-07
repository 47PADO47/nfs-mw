//! A whole car on the police simple tires (`cop-simple-chassis.md`).

use glam::Vec3;

use super::super::tests::{flat, parked, run};
use super::*;
use crate::FIXED_STEP;
use crate::input::InputState;
use crate::vehicle::SimpleChassisConfig;

fn police() -> Vehicle {
    let mut v = parked();
    v.set_simple_chassis(Some(SimpleChassisConfig {
        ride_extra: [-10.0, 0.0, 2.0],
        steering_range: [45.0, 60.0],
        landing_angular_retention: Vec3::new(1.0, 0.5, 1.0),
    }));
    v.set_simple_tires(true);
    v.config.automatic = true;
    v.config.auto_reverse = false;
    v
}

#[test]
fn a_police_car_on_the_simple_tires_pulls_away_straight_and_brakes_to_a_stop() {
    let mut v = police();
    run(&mut v, &InputState { throttle: 1.0, ..Default::default() }, 12.0);
    assert!(v.forward_speed() > 25.0, "it accelerates: {}", v.forward_speed());
    assert!(v.position().x.abs() < 0.5, "straight: x = {}", v.position().x);
    run(&mut v, &InputState { brake: 1.0, ..Default::default() }, 8.0);
    assert!(v.forward_speed().abs() < 1.0, "it stops: {}", v.forward_speed());
}

#[test]
fn a_steady_turn_needs_no_oversteer_correction() {
    let mut v = police();
    run(&mut v, &InputState { throttle: 1.0, ..Default::default() }, 8.0);
    run(&mut v, &InputState { throttle: 0.4, steer: 0.15, ..Default::default() }, 4.0);
    let up = v.body.rotation().y_axis;
    let yaw = v.body.angular_velocity.dot(up);
    assert!(yaw.abs() > 0.05, "the car turns: yaw rate {yaw}");
    let used = (v.body_slip - 0.25 * yaw).abs();
    let other = (v.body_slip + 0.25 * yaw).abs();
    assert!(used <= other, "the correction measures slip beyond the turn: {used} vs {other}");
}

#[test]
fn at_speed_a_steering_pulse_dies_out_without_weaving() {
    let mut v = police();
    v.place_moving(v.position(), v.orientation(), 60.0);
    run(&mut v, &InputState { throttle: 0.8, ..Default::default() }, 1.0);
    run(&mut v, &InputState { throttle: 0.8, steer: 0.1, ..Default::default() }, 0.3);
    let g = flat();
    let (mut flips, mut last) = (0, 0.0f32);
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &InputState { throttle: 0.8, ..Default::default() }, &g);
        let yaw = v.body.angular_velocity.dot(v.body.rotation().y_axis);
        if yaw.abs() > 0.01 && last != 0.0 && yaw.signum() != last.signum() {
            flips += 1;
        }
        if yaw.abs() > 0.01 {
            last = yaw;
        }
    }
    assert!(flips <= 1, "the yaw settles: {flips} sign changes");
    assert!(v.wheels_on_ground() == 4);
}
