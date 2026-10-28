//! The drift rule: the state machine on made-up steps, the unit conversions, and the real numbers of the 2012 install (set
//! `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`).

use std::path::Path;

use chameleon_bundle::Bundle;

use crate::drift::{DriftPayout, DriftPhysics, TANK_UNITS, read_drift_nitrous, read_drift_physics};
use crate::drift_model::{DriftInput, DriftModel, DriftParams, HIT_ENDS};

const DT: f32 = 1.0 / 60.0;

fn model() -> DriftModel {
    DriftModel::new(DriftParams::default())
}

/// A step of a good drift: fast, 20 degrees crooked, on the ground.
fn sliding() -> DriftInput {
    DriftInput { dt: DT, speed: 25.0, slip_angle: 20.0_f32.to_radians(), grounded: true, hit: 0.0 }
}

fn run(model: &mut DriftModel, input: &DriftInput, seconds: f32) -> f32 {
    let mut gained = 0.0;
    for _ in 0..(seconds / DT).round() as u32 {
        gained += model.step(input).gain;
    }
    gained
}

#[test]
fn the_numbers_are_converted_to_metres_radians_and_tank_fractions() {
    let params = DriftParams::default();
    assert_eq!(params.enter_speed, 10.0);
    assert!((params.enter_angle - 7.5_f32.to_radians()).abs() < 1e-6);
    assert!((params.exit_angle - 3.0_f32.to_radians()).abs() < 1e-6);
    assert_eq!(params.exit_hold, 0.25);
    assert!((params.pay_speed - 13.4112).abs() < 1e-3, "30 mph");
    assert_eq!(params.pay_delay, 0.75);
    assert!((params.tank_per_second - 0.06).abs() < 1e-6, "6 units of 100 a second");
    assert_eq!(DriftPayout::BURN.tank_per_second() * TANK_UNITS, 6.0);
}

#[test]
fn a_straight_car_does_not_drift_and_a_crooked_fast_one_does() {
    let mut model = model();
    let straight = DriftInput { slip_angle: 0.0, ..sliding() };
    assert!(!model.step(&straight).drifting);
    assert!(model.step(&sliding()).drifting);
    assert!(model.drifting());
}

#[test]
fn a_slide_needs_the_speed_the_angle_and_the_ground() {
    for input in [
        DriftInput { speed: 9.0, ..sliding() },
        DriftInput { slip_angle: 5.0_f32.to_radians(), ..sliding() },
        DriftInput { grounded: false, ..sliding() },
        DriftInput { speed: -25.0, slip_angle: 150.0_f32.to_radians(), ..sliding() },
        DriftInput { hit: 0.5, ..sliding() },
    ] {
        assert!(!model().step(&input).drifting, "{input:?}");
    }
}

#[test]
fn the_angle_counts_on_either_side() {
    let left = DriftInput { slip_angle: -20.0_f32.to_radians(), ..sliding() };
    assert!(model().step(&left).drifting);
}

#[test]
fn a_drift_pays_nothing_until_it_has_lasted_and_then_the_rate_of_the_data() {
    let mut model = model();
    let early = run(&mut model, &sliding(), 0.7);
    assert_eq!(early, 0.0);
    // 0.75 s in the drift pays; ten more seconds at 6 units of 100 a second.
    run(&mut model, &sliding(), 0.1);
    let paid = run(&mut model, &sliding(), 10.0);
    assert!((paid - 0.6).abs() < 0.01, "{paid}");
}

#[test]
fn a_slow_drift_does_not_pay() {
    let mut model = model();
    let slow = DriftInput { speed: 12.0, ..sliding() };
    assert!(model.step(&slow).drifting);
    assert_eq!(run(&mut model, &slow, 3.0), 0.0, "12 m/s is under 30 mph");
    assert!(model.drifting(), "it is still a drift");
}

#[test]
fn straightening_out_ends_the_drift_after_the_hold_time() {
    let mut model = model();
    run(&mut model, &sliding(), 2.0);
    let straight = DriftInput { slip_angle: 1.0_f32.to_radians(), ..sliding() };
    run(&mut model, &straight, 0.2);
    assert!(model.drifting(), "0.25 s of straight driving has not passed");
    let mut ended = None;
    for _ in 0..30 {
        ended = ended.or(model.step(&straight).ended);
    }
    let summary = ended.expect("the drift ended");
    assert!(!model.drifting());
    assert!(summary.seconds > 2.2 && summary.seconds < 2.6, "{}", summary.seconds);
    assert!(summary.distance > 50.0, "{}", summary.distance);
}

#[test]
fn a_short_straight_moment_does_not_end_the_drift() {
    let mut model = model();
    run(&mut model, &sliding(), 1.0);
    let straight = DriftInput { slip_angle: 0.0, ..sliding() };
    run(&mut model, &straight, 0.15);
    run(&mut model, &sliding(), 0.5);
    run(&mut model, &straight, 0.15);
    assert!(model.drifting(), "the hold time restarts when the car slides again");
}

#[test]
fn the_angle_between_exit_and_entry_keeps_a_drift_alive() {
    let mut model = model();
    run(&mut model, &sliding(), 1.0);
    let between = DriftInput { slip_angle: 5.0_f32.to_radians(), ..sliding() };
    run(&mut model, &between, 3.0);
    assert!(model.drifting(), "5 degrees is under the 7.5 to enter and over the 3 to exit");
    let out = model.step(&between);
    assert!(out.paying, "and it still pays");
}

#[test]
fn leaving_the_ground_or_slowing_down_ends_it_like_straightening_out() {
    for off in [DriftInput { grounded: false, ..sliding() }, DriftInput { speed: 8.0, ..sliding() }] {
        let mut model = model();
        run(&mut model, &sliding(), 1.0);
        run(&mut model, &off, 0.5);
        assert!(!model.drifting(), "{off:?}");
    }
}

#[test]
fn a_hit_ends_it_at_once_and_a_light_touch_does_not() {
    let mut model = model();
    run(&mut model, &sliding(), 1.0);
    assert!(model.step(&DriftInput { hit: HIT_ENDS / 2.0, ..sliding() }).drifting);
    let out = model.step(&DriftInput { hit: 0.5, ..sliding() });
    assert!(!out.drifting && out.ended.is_some());
    assert_eq!(out.gain, 0.0);
    assert!(!model.step(&DriftInput { hit: 0.5, ..sliding() }).drifting, "a hit does not start one");
}

#[test]
fn the_summary_adds_up_what_the_steps_gave() {
    let mut model = model();
    let mut earned = 0.0;
    let mut distance = 0.0;
    for _ in 0..180 {
        let out = model.step(&sliding());
        earned += out.gain;
        distance = out.distance;
    }
    let straight = DriftInput { slip_angle: 0.0, ..sliding() };
    let mut summary = None;
    for _ in 0..60 {
        summary = summary.or(model.step(&straight).ended);
    }
    let summary = summary.expect("ended");
    assert!((summary.earned - earned).abs() < 1e-6);
    assert!(summary.distance >= distance);
    assert!((distance - 25.0 * 3.0).abs() < 1.0, "{distance}");
}

#[test]
fn a_step_that_cannot_be_used_changes_nothing() {
    let mut model = model();
    run(&mut model, &sliding(), 1.0);
    for bad in [
        DriftInput { dt: 0.0, ..sliding() },
        DriftInput { dt: -1.0, ..sliding() },
        DriftInput { speed: f32::NAN, ..sliding() },
        DriftInput { slip_angle: f32::INFINITY, ..sliding() },
    ] {
        let out = model.step(&bad);
        assert!(out.drifting && out.gain == 0.0 && out.ended.is_none(), "{bad:?}");
    }
    assert!(model.drifting());
    model.reset();
    assert!(!model.drifting());
    assert_eq!(model.step(&DriftInput { dt: f32::NAN, ..sliding() }), Default::default());
}

#[test]
fn a_new_drift_can_follow_an_ended_one() {
    let mut model = model();
    run(&mut model, &sliding(), 1.0);
    run(&mut model, &DriftInput { slip_angle: 0.0, ..sliding() }, 0.5);
    assert!(!model.drifting());
    assert!(model.step(&sliding()).drifting);
    assert!(model.step(&sliding()).distance > 0.0);
}

fn install() -> Option<std::path::PathBuf> {
    std::env::var_os("CHAMELEON_GAME_DIR").map(Into::into)
}

fn car_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(dir.join("VEHICLES"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy().to_uppercase();
            name.starts_with("VEH_") && name.ends_with("_HI.BNDL")
        })
        .collect();
    files.sort();
    files
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_porsche_911_carrera_s_has_its_known_drift_numbers() {
    let Some(dir) = install() else { return };
    let data = std::fs::read(dir.join("VEHICLES/VEH_1085007_HI.BNDL")).unwrap();
    let bundle = Bundle::parse(&data).unwrap();
    let physics = read_drift_physics(&bundle).unwrap();
    assert_eq!(physics, DriftPhysics { trigger_speed: 10.0, trigger_angle: 7.0, exit_angle: 5.0, exit_hold: 0.25 });
    let nitrous = read_drift_nitrous(&bundle).unwrap();
    assert_eq!(nitrous.burn, DriftPayout::BURN);
    assert_eq!(nitrous.stock.gain, 5.0);
    assert!(!nitrous.stock.flag && nitrous.burn.flag);
    assert_eq!((nitrous.stock.message, nitrous.stock.min_speed_mph, nitrous.stock.delay), (24010, 30.0, 0.75));
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_car_of_the_install_has_the_same_payout_and_a_sensible_physics() {
    let Some(dir) = install() else { return };
    let files = car_files(&dir);
    assert_eq!(files.len(), 93);
    let mut angles = Vec::new();
    for file in &files {
        let data = std::fs::read(file).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let name = file.display();
        let nitrous = read_drift_nitrous(&bundle).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(nitrous.burn, DriftPayout::BURN, "{name}");
        assert_eq!(nitrous.stock.gain, 5.0, "{name}");
        let physics = read_drift_physics(&bundle).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!((5.0..=13.0).contains(&physics.trigger_angle), "{name}: {physics:?}");
        assert!((1.0..=6.0).contains(&physics.exit_angle), "{name}: {physics:?}");
        angles.push(physics.trigger_angle);
    }
    angles.sort_by(f32::total_cmp);
    assert_eq!(angles[angles.len() / 2], DriftPhysics::TYPICAL.trigger_angle, "the median trigger angle");
}
