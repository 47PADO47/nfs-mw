use super::{GEAR_FIRST, GEAR_REVERSE, Powertrain, tests, torque_loop::LoopCtx};
use crate::{engine::ClutchState, induction::InductionSpec, math::rpm_to_rad, nos::NosSpec};

#[test]
fn first_and_reverse_clutch_play_also_damps_an_engaging_flywheel() {
    for gear in [GEAR_FIRST, GEAR_REVERSE] {
        let mut engine = tests::engine();
        engine.torque.fill(0.0);
        let mut transmission = tests::trans();
        transmission.clutch_slip = 0.5;
        let mut p = Powertrain::new(engine, transmission, InductionSpec::default(), NosSpec::default(), 0.33);
        p.gear = gear;
        p.omega = rpm_to_rad(6000.0);
        p.clutch.disengage();
        p.clutch.engage(0.05);
        assert_eq!(p.clutch.state, ClutchState::Engaging);
        let previous = p.omega;
        let mut av = [0.0; 4];
        let dt = 1.0 / 144.0;
        p.torque_loop(&mut LoopCtx { dt, av: &mut av, grounded: [true; 4], drive_slip: 0.0, speed: 0.0 });
        // Difference exceeds 300 rad/s. Stiffness 20 and initial coupling 0.25
        // give road torque -1500. Clutch play adds -750 even while engaging.
        let expected = previous - 2250.0 / p.engine.inertia(false) * dt;
        assert!((p.omega - expected).abs() < 0.0001, "gear {gear}: {} vs {expected}", p.omega);
    }
}

#[test]
fn higher_gears_do_not_receive_first_gear_clutch_play() {
    let mut engine = tests::engine();
    engine.torque.fill(0.0);
    let mut p = Powertrain::new(engine, tests::trans(), InductionSpec::default(), NosSpec::default(), 0.33);
    p.gear = GEAR_FIRST + 1;
    p.omega = rpm_to_rad(6000.0);
    p.clutch.disengage();
    p.clutch.engage(0.25);
    let previous = p.omega;
    let mut av = [0.0; 4];
    let dt = 1.0 / 144.0;
    p.torque_loop(&mut LoopCtx { dt, av: &mut av, grounded: [true; 4], drive_slip: 0.0, speed: 0.0 });
    let expected = previous - 1500.0 / p.engine.inertia(false) * dt;
    assert!((p.omega - expected).abs() < 0.0001);
}

#[test]
fn opening_or_locking_the_clutch_clears_old_engagement_difference() {
    for engaged in [false, true] {
        let mut p =
            Powertrain::new(tests::engine(), tests::trans(), InductionSpec::default(), NosSpec::default(), 0.33);
        p.gear = GEAR_FIRST + 1;
        p.omega = rpm_to_rad(6000.0);
        p.prev_rpm_diff = 4000.0;
        if !engaged {
            p.clutch.disengage();
        }
        let mut av = [0.0; 4];
        p.torque_loop(&mut LoopCtx { dt: 1.0 / 144.0, av: &mut av, grounded: [true; 4], drive_slip: 0.0, speed: 0.0 });
        assert_eq!(p.prev_rpm_diff, 0.0, "engaged={engaged}");
        p.clutch.disengage();
        p.clutch.engage(0.25);
        p.omega = rpm_to_rad(6000.0);
        let mut fresh = p.clone();
        fresh.prev_rpm_diff = 0.0;
        let mut fresh_av = av;
        let actual = p.torque_loop(&mut LoopCtx {
            dt: 1.0 / 144.0,
            av: &mut av,
            grounded: [true; 4],
            drive_slip: 0.0,
            speed: 0.0,
        });
        let expected = fresh.torque_loop(&mut LoopCtx {
            dt: 1.0 / 144.0,
            av: &mut fresh_av,
            grounded: [true; 4],
            drive_slip: 0.0,
            speed: 0.0,
        });
        assert_eq!(actual, expected);
        assert_eq!(p.omega, fresh.omega);
        assert!(p.prev_rpm_diff < 0.0);
    }
}
