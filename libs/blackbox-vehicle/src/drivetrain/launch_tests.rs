use super::*;
use crate::drivetrain::tests::{engine, trans};
use crate::induction::InductionSpec;
use crate::math::rpm_to_rad;
use crate::nos::NosSpec;

fn drive(staging: bool, strength: f32, gas: f32, gear: usize) -> (f32, f32, bool) {
    let mut p = Powertrain::new(engine(), trans(), InductionSpec::default(), NosSpec::default(), 0.33);
    p.omega = rpm_to_rad(3000.0);
    p.gear = gear;
    p.throttle = gas;
    p.induction.boost = 3.0;
    p.nos.boost = 2.0;
    p.set_launch_context(staging, strength);
    let mut av = [0.0; 4];
    let torque = p.torque_loop(&mut super::torque_loop::LoopCtx {
        dt: 1.0 / 144.0,
        av: &mut av,
        grounded: [true; 4],
        loaded_wheels: 4,
        drive_slip: 0.0,
        speed: 0.0,
    });
    (torque, p.throttle, p.is_engine_braking())
}

#[test]
fn active_launch_replaces_torque_without_scaling_it_by_strength_or_induction() {
    let a = drive(false, 0.6, 0.2, GEAR_FIRST);
    let b = drive(false, 0.9, 1.0, GEAR_FIRST);
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, 1.0);
    assert!(!a.2);
    assert_ne!(a.0, drive(false, 0.0, 1.0, GEAR_FIRST).0);
    assert_eq!(drive(false, 0.9, 0.0, GEAR_FIRST), drive(false, 0.0, 0.0, GEAR_FIRST));
    assert_eq!(drive(true, 0.9, 0.2, GEAR_FIRST), drive(true, 0.0, 0.2, GEAR_FIRST));
    assert_eq!(drive(false, 0.6, 0.2, GEAR_REVERSE).1, 1.0, "the active override has no extra gear gate");
}

#[test]
fn staging_blocks_automatic_departure_from_neutral() {
    let mut p = Powertrain::new(engine(), trans(), InductionSpec::default(), NosSpec::default(), 0.33);
    p.gear = GEAR_NEUTRAL;
    p.set_launch_context(true, 0.0);
    let mut av = [0.0; 4];
    let mut i = TickInput {
        dt: 1.0 / 144.0,
        gas: 1.0,
        nos_held: false,
        can_recharge_nos: true,
        blown: false,
        automatic: true,
        speed: 0.0,
        wheel_av: &mut av,
        grounded: [true; 4],
        drifting: false,
        drive_slip: 0.0,
        max_wheel_slip: 0.0,
        induction_tuning: 0.0,
        nos_tuning: 0.0,
        clutch_pedal: None,
    };
    p.tick(&mut i);
    assert_eq!(p.gear(), GEAR_NEUTRAL);
    p.set_launch_context(false, 0.9);
    i.blown = true;
    p.tick(&mut i);
    assert_eq!(p.throttle, 0.0, "launch cannot restore a blown engine's throttle");
}
