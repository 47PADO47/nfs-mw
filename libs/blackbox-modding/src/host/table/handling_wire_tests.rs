//! The suspension and controls adapters of `bb.handling` in both directions: the query the vehicle builds reaches the mod's
//! callback field by field, and the mod's answer comes back as the vehicle's result (the steering flag included).

use core::mem::size_of;
use std::cell::Cell;
use std::rc::Rc;

use super::handling_hooks_tests::{
    ControlsScript, SuspensionScript, controls_query, controls_registration, fixture, good_controls, good_suspension,
    register_controls_hook, register_suspension_hook, suspension_query, suspension_registration,
};
use crate::abi::*;
use crate::host::test_support::callback_on_main;

#[test]
fn a_suspension_hook_reaches_the_backend_and_the_adapter_converts_both_ways() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = SuspensionScript::new(BB_SUSPENSION_HANDLED, good_suspension());
    let (status, id) = register_suspension_hook(&attached, &suspension_registration(&script));
    assert_eq!((status, id), (BB_OK, 1));
    let hook = fake.suspension.borrow()[0].1.clone();
    assert_eq!((hook.owner(), hook.stats().kind()), ("example.a", "suspension"));
    let mut model = hook.model(Rc::new(Cell::new(7)));
    let answer = model.axle(&suspension_query()).expect("handled");
    assert_eq!(
        (answer.wheels[0].body_force_n, answer.wheels[0].tyre_load_n, answer.wheels[0].camber_rad),
        (3400.0, 3700.0, -0.03)
    );
    assert_eq!(
        (answer.wheels[1].body_force_n, answer.wheels[1].tyre_load_n, answer.wheels[1].camber_rad),
        (-200.0, 3900.0, -0.04)
    );
    let seen = script.last.get();
    assert_eq!(seen.size as usize, size_of::<BbSuspensionQuery>());
    assert_eq!((seen.axle_index, seen.role, seen.vehicle_revision), (1, 0, 7));
    assert_eq!((seen.dt, seen.rest_travel_m, seen.max_travel_m, seen.corner_mass_kg), (1.0 / 60.0, 0.09, 0.15, 375.0));
    assert_eq!((seen.wheels[0].loaded, seen.wheels[0].compression_m), (1, 0.07));
    assert_eq!((seen.wheels[1].loaded, seen.wheels[1].compression_m), (0, 0.0));
    assert_eq!(seen.wheels[0].compression_speed_mps, 0.2);
    assert_eq!(seen.wheels[0].steer_angle_rad, 0.1);
    assert_eq!(seen.wheels[0].body_accel_up_mps2, -1.5);
    assert_eq!(seen.wheels[0].tyre_lateral_force_n, 900.0);
    assert_eq!(seen.wheels[0].static_load_n, 3700.0);
    assert_eq!(hook.stats().calls(), 1);
}

#[test]
fn a_controls_hook_reaches_the_backend_and_the_adapter_converts_both_ways() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let script = ControlsScript::new(BB_CONTROLS_HANDLED, good_controls());
    let (status, id) = register_controls_hook(&attached, &controls_registration(&script));
    assert_eq!((status, id), (BB_OK, 1));
    let hook = fake.controls.borrow()[0].1.clone();
    assert_eq!(hook.stats().kind(), "controls");
    let mut model = hook.model(Rc::new(Cell::new(3)));
    let answer = model.filter(&controls_query()).expect("handled");
    assert_eq!((answer.throttle, answer.brake, answer.steer_target), (0.4, [0.1, 0.2, 0.3, 0.4], Some(-0.25)));
    let seen = script.last.get();
    assert_eq!(seen.size as usize, size_of::<BbControlsQuery>());
    assert_eq!((seen.role, seen.vehicle_revision, seen.dt), (0, 3, 1.0 / 60.0));
    assert_eq!((seen.throttle, seen.brake, seen.handbrake, seen.steer), (0.9, 0.0, 0.2, -0.5));
    assert_eq!(
        (seen.forward_speed, seen.yaw_rate, seen.lateral_accel, seen.longitudinal_accel),
        (22.0, 0.3, 4.0, -2.0)
    );
    assert_eq!(seen.wheel_loaded_mask, 0b1101, "wheels 0, 2 and 3 are loaded");
    assert_eq!(seen.slip_ratio, [0.01, 0.0, -0.3, 0.2]);
    assert_eq!(seen.slip_angle, [0.02, 0.0, -0.01, 0.03]);
}

#[test]
fn a_controls_answer_without_the_steer_flag_leaves_the_steering_to_the_host() {
    let _scope = callback_on_main();
    let (fake, attached) = fixture();
    let mut result = good_controls();
    result.flags = 0;
    let script = ControlsScript::new(BB_CONTROLS_HANDLED, result);
    register_controls_hook(&attached, &controls_registration(&script));
    let hook = fake.controls.borrow()[0].1.clone();
    let mut model = hook.model(Rc::new(Cell::new(1)));
    let answer = model.filter(&controls_query()).expect("handled");
    assert_eq!(answer.steer_target, None, "the target is read only with the flag");
    assert_eq!(answer.throttle, 0.4);
}
