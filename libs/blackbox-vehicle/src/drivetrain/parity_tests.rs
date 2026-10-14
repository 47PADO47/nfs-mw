use super::*;
use crate::{engine::ClutchState, induction::InductionSpec, math::rpm_to_rad, nos::NosSpec};

#[test]
fn cached_peak_torque_matches_original_raw_sample_vectors() {
    for (torque, idle, max, redline, boost, peak, rpm) in [
        (vec![100.0, 150.0, 200.0, 250.0, 300.0], 1000.0, 9000.0, 7000.0, 0.0, 300.0, 7000.0),
        (vec![100.0, 150.0, 200.0, 250.0, 300.0], 1000.0, 9000.0, 7000.0, 0.4, 420.0, 7000.0),
        (vec![100.0, 200.0, 300.0, 220.0], 900.0, 8000.0, 7000.0, 0.0, 300.0, 5633.3335),
        (vec![], 900.0, 8000.0, 7000.0, 0.0, 0.0, 900.0),
        (vec![300.0], 900.0, 8000.0, 7000.0, 0.0, 0.0, 900.0),
    ] {
        let engine = crate::engine::EngineSpec { torque, idle, max_rpm: max, red_line: redline, ..tests::engine() };
        let induction = InductionSpec { low_boost: boost, high_boost: boost, ..Default::default() };
        let mut p = Powertrain::new(engine.clone(), tests::trans(), induction, NosSpec::default(), 0.33);
        assert!((p.peak_torque().0 - peak * crate::math::FT_LB_TO_NM).abs() < 0.001);
        assert!((p.peak_torque().1 - rpm).abs() < 0.002);
        p.reconfigure(engine, tests::trans(), induction, NosSpec::default(), 0.33);
        assert!((p.peak_torque().0 - peak * crate::math::FT_LB_TO_NM).abs() < 0.001);
        assert!((p.peak_torque().1 - rpm).abs() < 0.002);
    }
}

#[test]
fn hardware_shift_maps_match_original_untuned_induction_vectors() {
    let expected_up = [0.0, 0.0, 6900.0, 6900.0, 6950.0, 6750.0, 6550.0, 7000.0];
    let expected_down = [0.0, 0.0, 0.0, 4216.667, 4861.3633, 5156.4517, 5282.609, 5458.3335];
    for boost in [0.0, 0.4, 1.0] {
        let induction =
            InductionSpec { low_boost: boost, high_boost: boost, spool: 0.5, vacuum: -0.08, ..Default::default() };
        let powertrain = Powertrain::new(tests::engine(), tests::trans(), induction, NosSpec::default(), 0.33);
        assert_eq!(powertrain.shift_points().up, expected_up, "boost={boost}");
        for (&actual, expected) in powertrain.shift_points().down.iter().zip(expected_down) {
            assert!((actual - expected).abs() < 0.002, "boost={boost}: {actual} vs {expected}");
        }
    }
}

#[test]
fn clutch_reengagement_transfers_flywheel_torque_without_an_engine_peak_cap() {
    let mut p = Powertrain::new(tests::engine(), tests::trans(), InductionSpec::default(), NosSpec::default(), 0.33);
    p.gear = GEAR_FIRST + 1;
    p.omega = rpm_to_rad(6000.0);
    p.clutch.disengage();
    p.clutch.engage(0.25);
    assert_eq!(p.clutch.state, ClutchState::Engaging);
    let mut av = [0.0; 4];
    let torque = p.torque_loop(&mut super::torque_loop::LoopCtx {
        dt: crate::FIXED_STEP,
        av: &mut av,
        grounded: [true; 4],
        drive_slip: 0.0,
        speed: 0.0,
    });
    // Difference exceeds 300 rad/s: stiffness 20, initial coupling 0.25.
    let expected = 300.0 * 20.0 * 0.25 * 2.2 * 3.7 * 0.9;
    assert!((torque - expected).abs() < 0.01, "{torque} vs {expected}");
    assert!(torque > p.peak_torque_nm * 2.2 * 3.7 * 0.9);
}

#[test]
fn changing_hardware_retains_motion_state_and_rebuilds_cached_maps() {
    let tank = NosSpec { nos_capacity: 5.0, torque_boost: 0.2, ..NosSpec::default() };
    let mut p = Powertrain::new(tests::engine(), tests::trans(), InductionSpec::default(), tank, 0.33);
    p.match_speed(40.0);
    let (gear, rpm) = (p.gear(), p.rpm());
    p.nos.capacity = 0.3;
    p.nos.boost = 1.2;
    let mut engine = tests::engine();
    engine.torque.iter_mut().for_each(|t| *t *= 1.5);
    let peak = p.peak_torque().0;
    let new_tank = NosSpec { torque_boost: 0.3, ..tank };
    p.reconfigure(engine, tests::trans(), InductionSpec::default(), new_tank, 0.33);
    assert_eq!(p.gear(), gear);
    assert!((p.rpm() - rpm).abs() < 0.01);
    assert_eq!(p.nos.capacity, 0.3);
    assert_eq!(p.nos.boost, 1.0);
    assert_eq!(p.induction.boost, 0.0);
    assert!((p.peak_torque().0 / peak - 1.5).abs() < 0.001);
    p.reconfigure(tests::engine(), tests::trans(), InductionSpec::default(), NosSpec::default(), 0.33);
    assert!(!p.has_nos());
    assert_eq!(p.nos.capacity, 0.0);
}

fn tick_boosted(p: &mut Powertrain, held: bool) {
    let mut av = [35.0 / 0.33; 4];
    p.tick(&mut TickInput {
        dt: crate::FIXED_STEP,
        gas: 1.0,
        nos_held: held,
        can_recharge_nos: true,
        blown: false,
        automatic: false,
        speed: 35.0,
        wheel_av: &mut av,
        grounded: [true; 4],
        drifting: false,
        drive_slip: 0.0,
        max_wheel_slip: 0.0,
        induction_tuning: 0.0,
        nos_tuning: 0.0,
        clutch_pedal: None,
    });
}

#[test]
fn unrelated_hardware_preserves_running_induction_and_nitrous_burn_or_cooldown() {
    let induction = InductionSpec {
        low_boost: 0.2,
        high_boost: 0.4,
        spool: 0.3,
        spool_time_up: 2.0,
        spool_time_down: 1.0,
        psi: 20.0,
        ..InductionSpec::default()
    };
    let tank = NosSpec { nos_capacity: 8.0, torque_boost: 0.2, nos_disengage: 1.0, ..NosSpec::default() };
    for held in [true, false] {
        let mut p = Powertrain::new(tests::engine(), tests::trans(), induction, tank, 0.33);
        p.match_speed(35.0);
        for _ in 0..30 {
            tick_boosted(&mut p, true);
        }
        if !held {
            for _ in 0..10 {
                tick_boosted(&mut p, false);
            }
        }
        assert!(p.induction.spool > 0.0 && p.induction.boost > 0.0);
        assert!(p.nos.engaged > 0.0 && p.nos.capacity < 1.0);
        assert_eq!(p.nos.boost > 1.0, held);
        let before = (
            p.gear(),
            p.rpm(),
            p.nos.capacity,
            p.nos.engaged,
            p.nos.boost,
            p.induction.spool,
            p.induction.boost,
            p.induction.psi,
        );
        let speed = p.speed_at(5000.0, p.gear());
        // A tire-radius change exercises a real hardware replacement while boost specs stay equal.
        p.reconfigure(tests::engine(), tests::trans(), induction, tank, 0.34);
        assert_eq!(
            before,
            (
                p.gear(),
                p.rpm(),
                p.nos.capacity,
                p.nos.engaged,
                p.nos.boost,
                p.induction.spool,
                p.induction.boost,
                p.induction.psi
            )
        );
        assert!((p.speed_at(5000.0, p.gear()) / speed - 0.34 / 0.33).abs() < 1e-5);
        let changed = InductionSpec { high_boost: 0.8, ..induction };
        let old_peak = p.peak_torque().0;
        p.reconfigure(tests::engine(), tests::trans(), changed, tank, 0.34);
        assert_eq!((p.induction.spool, p.induction.boost, p.induction.psi), (0.0, 0.0, 0.0));
        assert_eq!((p.nos.capacity, p.nos.engaged, p.nos.boost), (before.2, before.3, before.4));
        assert!(p.peak_torque().0 > old_peak, "new induction updates the full-boost torque map");
        let changed_tank = NosSpec { torque_boost: 0.4, ..tank };
        p.reconfigure(tests::engine(), tests::trans(), changed, changed_tank, 0.34);
        assert_eq!(p.nos.capacity, before.2);
        assert_eq!((p.nos.engaged, p.nos.boost), (0.0, 1.0));
    }
}
