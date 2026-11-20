//! The scenario harness: the planar reference car of docs/specs/ac-handling/scenarios.md (appendix) runs the six scenarios on
//! the model, and the scalars are checked against targets.md. No game and no install are needed.
//!
//! Two checks per scalar of `ref_sport`: the `design` band (the model must feel like a plausible sport-tyre car) and the
//! `planar` regression value to 3 %. The Assetto Corsa reference of every row is `unmeasured` in targets.md, so that
//! comparison is skipped with a printed reason and nothing is guessed. Run with `-- --nocapture` to see the report.

mod metrics;
mod planar;
mod script;
mod targets;

use ac_handling::mapping::{CarParams, CarSpec, MapOptions, map_car};
use ac_handling::presets::{PresetSet, parse_car_file};

use metrics::{Rig, V100};
use targets::{CLASSES, REF_SPORT, Target, planar_tolerance};

const DT_60: f64 = 1.0 / 60.0;

fn presets() -> PresetSet {
    PresetSet::builtin().expect("the shipped presets parse")
}

/// The wheel inertia scale of the spec's planar numbers (wheel-dynamics.md section 5). The shipped presets use 1.0 so that the
/// game's drivetrain stays stable (NOTES.md); the planar car has no drivetrain compliance and the targets were computed with
/// 0.3, so every rig pins it.
const PLANAR_INERTIA_SCALE: f64 = 0.3;

fn rig_with(spec: CarSpec, car_file: Option<&str>, dt: f64) -> Rig {
    let mut file = car_file.map(|text| parse_car_file(text).expect("a valid car file")).unwrap_or_default();
    file.both.inertia_scale = Some(PLANAR_INERTIA_SCALE);
    let options = MapOptions { car: Some(&file), forced: None };
    let params: CarParams = map_car(&spec, &presets(), &options);
    Rig { spec, params, dt }
}

fn ref_rig(dt: f64) -> Rig {
    rig_with(CarSpec::ref_sport(), None, dt)
}

/// The reference car on a compound at its reference grip level (targets.md section 3): `rho = 0.89`.
fn class_rig(level: f64) -> Rig {
    let peak = level * 2.075;
    let spec = CarSpec { peak_friction: [peak; 2], sliding_friction: [0.89 * peak; 2], ..CarSpec::ref_sport() };
    rig_with(spec, None, DT_60)
}

/// The ten scalars of section 2.1, in the order of [`REF_SPORT`].
fn ref_scalars(rig: &Rig) -> [f64; 10] {
    let braking = metrics::braking(rig).expect("ref_sport reaches 100 km/h");
    let step = metrics::step_steer(rig, 0.06);
    let slide = metrics::slide(rig);
    let launch = metrics::launch(rig);
    [
        metrics::skidpad_g(rig),
        braking.distance_m,
        braking.peak_g,
        metrics::understeer(rig),
        step.overshoot_pct,
        step.settle_s,
        slide.breakaway.unwrap_or(f64::NAN),
        slide.recover_s.unwrap_or(f64::NAN),
        launch.d3_m,
        launch.t100_s.unwrap_or(f64::NAN),
    ]
}

/// Print the line of a scalar and say why the reference comparison is skipped.
fn report(target: &Target, got: f64) {
    let reference = match target.ac {
        Some(value) => format!("AC reference {value}"),
        None => "AC reference unmeasured: comparison skipped".to_owned(),
    };
    eprintln!(
        "{}: {:.4} {} (design {} in {:?}, planar {}; {reference})",
        target.name, got, target.unit, target.design, target.band, target.planar
    );
}

#[test]
fn the_lead_in_times_of_the_planar_car_match_the_spec() {
    let rig = ref_rig(DT_60);
    let t100 = rig.lead(V100).expect("reaches 100 km/h");
    let t72 = rig.lead(20.0).expect("reaches 20 m/s");
    eprintln!("T_LEAD(27.78) = {t100}, T_LEAD(20) = {t72}");
    assert!((t100 - 4.98).abs() <= 0.05, "T_LEAD(27.78) = {t100}");
    assert!((t72 - 3.57).abs() <= 0.05, "T_LEAD(20) = {t72}");
}

#[test]
fn ref_sport_is_inside_the_design_bands_of_targets_md() {
    let scalars = ref_scalars(&ref_rig(DT_60));
    let mut failures = Vec::new();
    for (target, got) in REF_SPORT.iter().zip(scalars) {
        report(target, got);
        let (lo, hi) = target.band;
        if !(lo..=hi).contains(&got) {
            failures.push(format!("{} = {got} is outside {lo} to {hi}", target.name));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn ref_sport_matches_the_planar_regression_values() {
    let scalars = ref_scalars(&ref_rig(DT_60));
    let mut failures = Vec::new();
    for (target, got) in REF_SPORT.iter().zip(scalars) {
        let allowed = planar_tolerance(target.planar, target.unit);
        if !got.is_finite() || (got - target.planar).abs() > allowed {
            failures.push(format!("{} = {got}, planar {} +/- {allowed}", target.name, target.planar));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn the_raw_braking_distance_is_normalised_to_100_kmh_as_the_spec_says() {
    // scenarios.md section 6: raw 34.95 m from 100.15 km/h, normalised to 34.85 m (the spec's figures are rounded).
    let braking = metrics::braking(&ref_rig(DT_60)).expect("reaches 100 km/h");
    eprintln!("S2 raw {:.2} m from {:.2} km/h", braking.raw_m, braking.v0 * 3.6);
    assert!((braking.raw_m - 34.95).abs() <= 0.1, "{}", braking.raw_m);
    assert!((braking.v0 * 3.6 - 100.15).abs() <= 0.1, "{}", braking.v0 * 3.6);
    assert!((braking.raw_m * (V100 / braking.v0).powi(2) - braking.distance_m).abs() < 1e-9);
}

#[test]
fn the_slide_ladder_of_s5_matches_the_planar_rungs() {
    let slide = metrics::slide(&ref_rig(DT_60));
    for rung in &slide.rungs {
        eprintln!(
            "S5 rung {}: beta_peak {:.1} deg, recovery {:?}{}",
            rung.throttle,
            rung.beta_peak_deg,
            rung.recovery_s,
            if rung.spun() { " (spun)" } else { "" }
        );
    }
    // targets.md planar rungs: 0.5 and 0.6 stay under 5 degrees and recover fast, 0.7 slides and recovers, 0.8 and up spin.
    assert!(slide.rungs[0].beta_peak_deg < 5.0 && slide.rungs[1].beta_peak_deg < 5.0);
    assert!(slide.rungs[2].beta_peak_deg >= 5.0 && !slide.rungs[2].spun());
    assert!(slide.rungs[3..].iter().all(|rung| rung.spun()));
    assert_eq!(slide.breakaway, Some(0.7));
}

#[test]
fn the_steady_lateral_acceleration_of_s4_is_in_the_window_the_spec_asks_for() {
    let step = metrics::step_steer(&ref_rig(DT_60), 0.06);
    eprintln!("S4 r_ss {:.4} rad/s, steady ay {:.3} g", step.r_ss, step.ay_g);
    assert!((0.45..=0.75).contains(&step.ay_g), "{}", step.ay_g);
    assert!((step.r_ss - 0.2055).abs() <= 0.03 * 0.2055 + 0.003, "{}", step.r_ss);
}

#[test]
fn the_understeer_staircase_stays_near_22_metres_per_second() {
    let (gradient, speeds) = metrics::understeer_with(&ref_rig(DT_60), 0.09);
    eprintln!("S3 gradient {gradient:.3} deg/g at speeds {speeds:?}");
    assert!(!speeds.is_empty(), "some step landed between 0.1 and 0.5 g");
    assert!(speeds.iter().all(|v| (18.0..=26.0).contains(v)), "{speeds:?}");
}

#[test]
fn the_compound_classes_match_section_3_of_targets_md() {
    let mut failures = Vec::new();
    let mut check = |name: &str, got: f64, want: f64, unit: &str| {
        let allowed = planar_tolerance(want, unit);
        eprintln!("{name}: {got:.3} (planar {want}, +/- {allowed:.3})");
        if !got.is_finite() || (got - want).abs() > allowed {
            failures.push(format!("{name} = {got}, planar {want} +/- {allowed}"));
        }
    };
    for row in CLASSES {
        let rig = class_rig(row.level);
        let name = |scalar: &str| format!("{} {scalar}", row.compound);
        check(&name("skidpad_g"), metrics::skidpad_g(&rig), row.skidpad_g, "g");
        let braking = metrics::braking(&rig).expect("reaches 100 km/h");
        check(&name("brake_m"), braking.distance_m, row.brake_m, "m");
        check(&name("brake_peak_g"), braking.peak_g, row.brake_peak_g, "g");
        check(&name("understeer"), metrics::understeer(&rig), row.understeer, "deg/g");
        let step = metrics::step_steer(&rig, 0.06);
        check(&name("overshoot"), step.overshoot_pct, row.overshoot_pct, "%");
        check(&name("settle"), step.settle_s, row.settle_s, "s");
        check(&name("launch_d3"), metrics::launch(&rig).d3_m, row.launch_d3_m, "m");
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn the_economy_compound_does_not_reach_100_kmh_inside_the_6_seconds_of_s6() {
    let launch = metrics::launch(&class_rig(0.60));
    eprintln!("economy launch: d3 {:.2} m, t100 {:?}", launch.d3_m, launch.t100_s);
    assert_eq!(launch.t100_s, None);
}

#[test]
fn grip_rises_and_understeer_falls_from_economy_to_slick() {
    let rows: Vec<(f64, f64)> = CLASSES
        .iter()
        .map(|row| (metrics::skidpad_g(&class_rig(row.level)), metrics::understeer(&class_rig(row.level))))
        .collect();
    for pair in rows.windows(2) {
        assert!(pair[1].0 > pair[0].0, "skidpad_g must increase: {rows:?}");
        assert!(pair[1].1 < pair[0].1, "understeer must decrease: {rows:?}");
    }
}

#[test]
fn between_the_scalars_the_properties_of_targets_md_hold() {
    let rig = ref_rig(DT_60);
    let braking = metrics::braking(&rig).expect("reaches 100 km/h");
    assert!(braking.peak_g >= metrics::skidpad_g(&rig), "a tyre grips more straight than sideways");
    assert!(metrics::understeer(&rig) > 0.0);
}

#[test]
fn the_144_hz_tick_agrees_with_60_hz_on_skidpad_and_braking() {
    let (slow, fast) = (ref_rig(DT_60), ref_rig(1.0 / 144.0));
    let (g60, g144) = (metrics::skidpad_g(&slow), metrics::skidpad_g(&fast));
    let (b60, b144) = (metrics::braking(&slow).expect("60"), metrics::braking(&fast).expect("144"));
    eprintln!("skidpad {g60:.4} / {g144:.4}, brake {:.2} / {:.2} m", b60.distance_m, b144.distance_m);
    assert!((g144 - g60).abs() <= 0.02 * g60, "{g60} against {g144}");
    assert!((b144.distance_m - b60.distance_m).abs() <= 0.02 * b60.distance_m);
}

#[test]
fn sanity_a_car_that_cannot_move_has_no_skidpad_and_no_braking_distance() {
    let spec = CarSpec { max_power_w: 0.0, ..CarSpec::ref_sport() };
    let rig = rig_with(spec, None, DT_60);
    assert_eq!(metrics::skidpad_g(&rig), 0.0);
    assert!(metrics::braking(&rig).is_none());
}

#[test]
fn sanity_identical_axles_understeer_much_less() {
    let flat = rig_with(CarSpec::ref_sport(), Some("schema = 1\naxle_angle_bias_deg = 0\n"), DT_60);
    let gradient = metrics::understeer(&flat);
    eprintln!("axle_angle_bias_deg = 0: {gradient:.3} deg/g");
    assert!(gradient < 0.5, "{gradient}");
}

#[test]
fn sanity_doubling_mu_y0_raises_grip_by_less_than_a_factor_of_two() {
    let low = rig_with(CarSpec::ref_sport(), Some("schema = 1\n[tyre]\nmu_y0 = 1.0\n"), DT_60);
    let high = rig_with(CarSpec::ref_sport(), Some("schema = 1\n[tyre]\nmu_y0 = 2.0\n"), DT_60);
    let (skid_low, skid_high) = (metrics::skidpad_g(&low), metrics::skidpad_g(&high));
    let (brake_low, brake_high) =
        (metrics::braking(&low).expect("low").peak_g, metrics::braking(&high).expect("high").peak_g);
    eprintln!("skidpad {skid_low:.3} -> {skid_high:.3}, brake peak {brake_low:.3} -> {brake_high:.3}");
    assert!(skid_high > skid_low && skid_high < 2.0 * skid_low);
    assert!(brake_high > brake_low && brake_high < 2.0 * brake_low);
}
