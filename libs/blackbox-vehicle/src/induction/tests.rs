use super::*;

fn turbo() -> InductionSpec {
    InductionSpec {
        low_boost: 0.0,
        high_boost: 0.5,
        spool: 0.4,
        spool_time_up: 1.0,
        spool_time_down: 0.5,
        vacuum: -0.1,
        psi: 15.0,
    }
}

fn input(rpm: f32, throttle: f32) -> InductionInput {
    InductionInput { dt: 1.0 / 60.0, throttle, rpm, idle: 1000.0, red_line: 7000.0, shifting: false, tuning: 0.0 }
}

#[test]
fn kinds() {
    assert_eq!(InductionSpec::default().kind(), InductionKind::None);
    assert_eq!(turbo().kind(), InductionKind::Turbo);
    assert_eq!(InductionSpec { spool: 0.0, ..turbo() }.kind(), InductionKind::Supercharger);
}

#[test]
fn naturally_aspirated_stays_zero() {
    let mut i = Induction::default();
    i.update(&InductionSpec::default(), &input(5000.0, 1.0));
    assert_eq!((i.spool, i.boost, i.psi), (0.0, 0.0, 0.0));
}

#[test]
fn turbo_spools_up_over_its_time_and_boosts() {
    let spec = turbo();
    let mut i = Induction::default();
    for _ in 0..30 {
        i.update(&spec, &input(6500.0, 1.0));
    }
    assert!((i.spool - 0.5).abs() < 0.02, "spool = {}", i.spool);
    for _ in 0..40 {
        i.update(&spec, &input(6500.0, 1.0));
    }
    assert_eq!(i.spool, 1.0);
    assert!(i.boost > 0.3 && i.boost <= 0.5, "boost = {}", i.boost);
    assert!(i.psi > 5.0 && i.psi <= 15.0);
}

#[test]
fn turbo_does_not_spool_below_its_rpm() {
    let spec = turbo();
    let mut i = Induction::default();
    for _ in 0..120 {
        i.update(&spec, &input(2000.0, 1.0));
    }
    assert_eq!(i.spool, 0.0);
}

#[test]
fn lifting_the_throttle_spools_down() {
    let spec = turbo();
    let mut i = Induction::default();
    for _ in 0..90 {
        i.update(&spec, &input(6500.0, 1.0));
    }
    for _ in 0..40 {
        i.update(&spec, &input(6500.0, 0.0));
    }
    assert_eq!(i.spool, 0.0);
}

#[test]
fn supercharger_boosts_across_the_range() {
    let spec = InductionSpec { spool: 0.0, spool_time_up: 0.0, spool_time_down: 0.0, ..turbo() };
    let mut i = Induction::default();
    i.update(&spec, &input(1500.0, 1.0));
    assert_eq!(i.spool, 1.0);
}

#[test]
fn full_boost_is_zero_for_none() {
    assert_eq!(InductionSpec::default().full_boost(5000.0, 1000.0, 7000.0), 0.0);
    assert!(turbo().full_boost(7000.0, 1000.0, 7000.0) > 0.4);
}
