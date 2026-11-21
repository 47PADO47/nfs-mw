//! The scenario harness: the planar reference car of docs/specs/ac-handling/scenarios.md (appendix, convention E) runs the six
//! scenarios on the model, and the scalars are checked against targets.md. No game and no install are needed.
//!
//! Two checks per scalar of `ref_sport`: the `design` band (the model must feel like a plausible sport-tyre car) and the
//! `planar` regression value to 3 % (0.05 s, 0.5 m). The Assetto Corsa reference of every row is `unmeasured` in targets.md, so
//! that comparison is skipped with a printed reason and nothing is guessed. The inertia sweep of scenarios.md section 6 and the
//! compound tables of targets.md section 3 are reproduced too. Run with `-- --nocapture` to see the report.

mod metrics;
mod planar;
mod script;
mod targets;

use ac_handling::mapping::{CarParams, CarSpec, MapOptions, map_car};
use ac_handling::presets::{PresetSet, parse_car_file};

use metrics::{Rig, Scalars, V100, with_brakes};
use targets::{CLASSES, REF_SPORT, SWEEP, Target, planar_tolerance};

const DT_60: f64 = 1.0 / 60.0;

fn presets() -> PresetSet {
    PresetSet::builtin().expect("the shipped presets parse")
}

fn rig_with(spec: CarSpec, car_file: Option<&str>, dt: f64) -> Rig {
    let file = car_file.map(|text| parse_car_file(text).expect("a valid car file"));
    let options = MapOptions { car: file.as_ref(), forced: None };
    let params: CarParams = map_car(&spec, &presets(), &options);
    Rig { spec, params, dt }
}

fn ref_rig(dt: f64) -> Rig {
    rig_with(CarSpec::ref_sport(), None, dt)
}

/// The reference car with the key `inertia_scale` set (the default is 1.0).
fn inertia_rig(scale: f64) -> Rig {
    rig_with(CarSpec::ref_sport(), Some(&format!("schema = 1\n[tyre]\ninertia_scale = {scale}\n")), DT_60)
}

/// The reference car on a compound at its reference grip level (targets.md section 3): `rho = 0.89`.
fn class_rig(level: f64) -> Rig {
    let peak = level * 2.075;
    let spec = CarSpec { peak_friction: [peak; 2], sliding_friction: [0.89 * peak; 2], ..CarSpec::ref_sport() };
    rig_with(spec, None, DT_60)
}

/// The ten scalars of section 2.1, in the order of [`REF_SPORT`].
fn ref_scalars(scalars: &Scalars) -> [f64; 10] {
    [
        scalars.skidpad_g,
        scalars.brake_m,
        scalars.brake_peak_g,
        scalars.understeer,
        scalars.step.overshoot_pct,
        scalars.step.settle_s,
        scalars.slide.breakaway.unwrap_or(f64::NAN),
        scalars.slide.recover_s.unwrap_or(f64::NAN),
        scalars.launch.d3_m,
        scalars.launch.t100_s.unwrap_or(f64::NAN),
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

/// Whether `got` is within the planar tolerance of `want`; a number that is not finite never is.
fn near(got: f64, want: f64, unit: &str) -> bool {
    got.is_finite() && (got - want).abs() <= planar_tolerance(want, unit)
}

#[test]
fn the_lead_in_times_of_the_planar_car_match_the_spec() {
    let rig = ref_rig(DT_60);
    let t100 = rig.lead(V100).expect("reaches 100 km/h");
    let t72 = rig.lead(20.0).expect("reaches 20 m/s");
    eprintln!("T_LEAD(27.78) = {t100}, T_LEAD(20) = {t72}");
    assert!((t100 - 5.52).abs() <= 0.05, "T_LEAD(27.78) = {t100}");
    assert!((t72 - 3.87).abs() <= 0.05, "T_LEAD(20) = {t72}");
    let light = inertia_rig(0.3);
    assert!((light.lead(V100).expect("reaches") - 4.98).abs() <= 0.05);
    assert!((light.lead(20.0).expect("reaches") - 3.57).abs() <= 0.05);
}

#[test]
fn ref_sport_is_inside_the_design_bands_and_on_the_planar_values() {
    let scalars = ref_scalars(&metrics::all(&ref_rig(DT_60)));
    let mut failures = Vec::new();
    for (target, got) in REF_SPORT.iter().zip(scalars) {
        report(target, got);
        let (lo, hi) = target.band;
        if !(lo..=hi).contains(&got) {
            failures.push(format!("{} = {got} is outside {lo} to {hi}", target.name));
        }
        if !near(got, target.planar, target.unit) {
            failures.push(format!("{} = {got}, planar {}", target.name, target.planar));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn the_raw_braking_distance_is_normalised_to_100_kmh_and_matches_the_closed_form() {
    // scenarios.md section 6: raw 41.10 m from 100.24 km/h, normalised to 40.91 m; targets.md check 6: within 2 % of 41.0 m.
    let braking = metrics::braking(&ref_rig(DT_60)).expect("reaches 100 km/h");
    eprintln!("S2 raw {:.2} m from {:.2} km/h", braking.raw_m, braking.v0 * 3.6);
    assert!((braking.raw_m - 41.10).abs() <= 0.2, "{}", braking.raw_m);
    assert!((braking.v0 * 3.6 - 100.24).abs() <= 0.1, "{}", braking.v0 * 3.6);
    assert!((braking.raw_m * (V100 / braking.v0).powi(2) - braking.distance_m).abs() < 1e-9);
    assert!((braking.distance_m - 41.0).abs() <= 0.02 * 41.0, "{}", braking.distance_m);
}

#[test]
fn s2t_with_stronger_brakes_the_tyre_is_the_limit() {
    let braking = metrics::braking(&with_brakes(&ref_rig(DT_60), 1.3)).expect("reaches 100 km/h");
    eprintln!("S2t: {:.2} m, {:.3} g", braking.distance_m, braking.peak_g);
    assert!(near(braking.distance_m, 37.65, "m") && near(braking.peak_g, 1.189, "g"), "{braking:?}");
}

#[test]
fn the_slide_ladder_of_s5_matches_the_planar_rungs() {
    let slide = metrics::slide(&ref_rig(DT_60));
    // throttle: (beta_peak deg, recovery s or None when spun)
    let want = [
        (0.50, 1.7, Some(0.22)),
        (0.55, 1.9, Some(0.22)),
        (0.60, 2.1, Some(0.22)),
        (0.65, 2.5, Some(0.23)),
        (0.70, 3.9, Some(0.33)),
        (0.75, 5.9, Some(0.62)),
        (0.80, 10.5, Some(1.87)),
        (0.85, 33.1, Some(3.90)),
        (0.90, 48.8, None),
        (0.95, 63.7, None),
        (1.00, 79.5, None),
    ];
    assert_eq!(slide.rungs.len(), 11);
    for (rung, (throttle, beta, recovery)) in slide.rungs.iter().zip(want) {
        eprintln!(
            "S5 rung {throttle:.2}: beta_peak {:.1} deg, recovery {:?}{}",
            rung.beta_peak_deg,
            rung.recovery_s,
            if rung.spun() { " (spun)" } else { "" }
        );
        assert!((rung.throttle - throttle).abs() < 1e-9);
        assert!((rung.beta_peak_deg - beta).abs() <= 0.03 * beta + 0.15, "rung {throttle}: {}", rung.beta_peak_deg);
        match recovery {
            Some(seconds) => assert!(
                rung.recovery_s.is_some_and(|got| (got - seconds).abs() <= 0.05 + 0.03 * seconds),
                "rung {throttle}: {:?}",
                rung.recovery_s
            ),
            None => assert!(rung.spun(), "rung {throttle} spins"),
        }
    }
    assert!(slide.breakaway.is_some_and(|rung| (rung - 0.75).abs() < 1e-9));
    assert!(slide.recover_s.is_some_and(|seconds| (seconds - 0.62).abs() <= 0.05));
}

#[test]
fn the_steady_lateral_acceleration_of_s4_is_in_the_window_the_spec_asks_for() {
    let step = metrics::step_steer(&ref_rig(DT_60), 0.06);
    eprintln!("S4 r_ss {:.4} rad/s, steady ay {:.3} g", step.r_ss, step.ay_g);
    assert!((0.45..=0.75).contains(&step.ay_g), "{}", step.ay_g);
    assert!((step.r_ss - 0.2058).abs() <= 0.03 * 0.2058, "{}", step.r_ss);
    assert!((step.ay_g - 0.598).abs() <= 0.03 * 0.598, "{}", step.ay_g);
}

#[test]
fn the_understeer_staircase_stays_near_22_metres_per_second() {
    let (gradient, speeds) = metrics::understeer_with(&ref_rig(DT_60), 0.09);
    eprintln!("S3 gradient {gradient:.3} deg/g at speeds {speeds:?}");
    assert!(!speeds.is_empty(), "some step landed between 0.1 and 0.5 g");
    assert!(speeds.iter().all(|v| (18.0..=26.0).contains(v)), "{speeds:?}");
}

#[test]
fn the_inertia_sweep_of_scenarios_md_section_6() {
    let mut failures = Vec::new();
    for row in SWEEP {
        let got = metrics::all(&inertia_rig(row.scale));
        eprintln!(
            "inertia {}: skid {:.3} brake {:.2}/{:.3} us {:.3} os {:.2} settle {:.3} breakaway {:?} recover {:?} d3 {:.2} t100 {:?} lead20 {:.2}",
            row.scale,
            got.skidpad_g,
            got.brake_m,
            got.brake_peak_g,
            got.understeer,
            got.step.overshoot_pct,
            got.step.settle_s,
            got.slide.breakaway,
            got.slide.recover_s,
            got.launch.d3_m,
            got.launch.t100_s,
            got.lead_20
        );
        let mut check = |name: &str, value: f64, want: f64, unit: &str| {
            if !near(value, want, unit) {
                failures.push(format!("inertia {} {name} = {value}, want {want}", row.scale));
            }
        };
        check("skidpad_g", got.skidpad_g, row.skidpad_g, "g");
        check("brake_m", got.brake_m, row.brake_m, "m");
        check("brake_peak_g", got.brake_peak_g, row.brake_peak_g, "g");
        check("understeer", got.understeer, row.understeer, "deg/g");
        check("overshoot", got.step.overshoot_pct, row.overshoot_pct, "%");
        check("settle", got.step.settle_s, row.settle_s, "s");
        check("breakaway", got.slide.breakaway.unwrap_or(f64::NAN), row.breakaway, "");
        check("recover", got.slide.recover_s.unwrap_or(f64::NAN), row.recover_s, "s");
        check("d3", got.launch.d3_m, row.d3_m, "m");
        check("lead_20", got.lead_20, row.lead_20, "s");
        match (got.launch.t100_s, row.t100_s) {
            (Some(value), Some(want)) => check("t100", value, want, "s"),
            (None, None) => {}
            (value, want) => failures.push(format!("inertia {} t100 = {value:?}, want {want:?}", row.scale)),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn the_compound_classes_match_section_3_of_targets_md() {
    let mut failures = Vec::new();
    let mut check = |name: &str, got: f64, want: f64, unit: &str| {
        eprintln!("{name}: {got:.3} (planar {want}, +/- {:.3})", planar_tolerance(want, unit));
        if !near(got, want, unit) {
            failures.push(format!("{name} = {got}, planar {want}"));
        }
    };
    for row in CLASSES {
        let rig = class_rig(row.level);
        let name = |scalar: &str| format!("{} {scalar}", row.compound);
        check(&name("skidpad_g"), metrics::skidpad_g(&rig), row.skidpad_g, "g");
        let braking = metrics::braking(&rig).expect("reaches 100 km/h");
        check(&name("brake_m"), braking.distance_m, row.brake_m, "m");
        check(&name("brake_peak_g"), braking.peak_g, row.brake_peak_g, "g");
        let strong = metrics::braking(&with_brakes(&rig, 1.3)).expect("reaches 100 km/h");
        check(&name("S2t brake_m"), strong.distance_m, row.s2t_brake_m, "m");
        check(&name("S2t brake_peak_g"), strong.peak_g, row.s2t_peak_g, "g");
        check(&name("understeer"), metrics::understeer(&rig), row.understeer, "deg/g");
        let step = metrics::step_steer(&rig, 0.06);
        check(&name("overshoot"), step.overshoot_pct, row.overshoot_pct, "%");
        check(&name("settle"), step.settle_s, row.settle_s, "s");
        check(&name("launch_d3"), metrics::launch(&rig).d3_m, row.launch_d3_m, "m");
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn only_the_three_grippiest_compounds_reach_100_kmh_inside_the_6_seconds_of_s6() {
    let reaches: Vec<Option<f64>> = CLASSES.iter().map(|row| metrics::launch(&class_rig(row.level)).t100_s).collect();
    eprintln!("launch_t100_s by compound: {reaches:?}");
    assert_eq!((reaches[0], reaches[1]), (None, None), "economy and road do not");
    for (found, want) in reaches[2..].iter().zip([5.517, 4.95, 4.35]) {
        assert!(found.is_some_and(|value| near(value, want, "s")), "{found:?} against {want}");
    }
    let economy = class_rig(0.60).lead(V100).expect("it gets there, slowly");
    let road = class_rig(0.80).lead(V100).expect("it gets there, slowly");
    assert!(near(economy, 6.90, "s") && near(road, 6.07, "s"), "{economy} {road}");
}

#[test]
fn the_class_rows_are_monotone() {
    let rows: Vec<(f64, f64, f64, f64)> = CLASSES
        .iter()
        .map(|row| {
            let rig = class_rig(row.level);
            let strong = metrics::braking(&with_brakes(&rig, 1.3)).expect("brakes");
            (metrics::skidpad_g(&rig), metrics::understeer(&rig), strong.distance_m, strong.peak_g)
        })
        .collect();
    for pair in rows.windows(2) {
        assert!(pair[1].0 > pair[0].0, "skidpad_g must increase: {rows:?}");
        assert!(pair[1].1 < pair[0].1, "understeer must decrease: {rows:?}");
        assert!(pair[1].2 < pair[0].2, "S2t brake_m must fall: {rows:?}");
        assert!(pair[1].3 > pair[0].3, "S2t brake_peak_g must rise: {rows:?}");
    }
}

#[test]
fn between_the_scalars_the_properties_of_targets_md_hold() {
    let rig = ref_rig(DT_60);
    let braking = metrics::braking(&rig).expect("reaches 100 km/h");
    assert!(braking.peak_g >= metrics::skidpad_g(&rig) || braking.peak_g > 1.0, "stock brakes: {}", braking.peak_g);
    let strong = metrics::braking(&with_brakes(&rig, 1.3)).expect("brakes");
    assert!(strong.peak_g >= metrics::skidpad_g(&rig), "a tyre grips more straight than sideways (S2t)");
    assert!(metrics::understeer(&rig) > 0.0);
}

#[test]
fn the_144_hz_tick_agrees_with_60_hz() {
    let (slow, fast) = (ref_rig(DT_60), ref_rig(1.0 / 144.0));
    let (g60, g144) = (metrics::skidpad_g(&slow), metrics::skidpad_g(&fast));
    let (b60, b144) = (metrics::braking(&slow).expect("60"), metrics::braking(&fast).expect("144"));
    eprintln!("skidpad {g60:.4} / {g144:.4}, brake {:.2} / {:.2} m", b60.distance_m, b144.distance_m);
    assert!((g144 - g60).abs() <= 0.02 * g60, "{g60} against {g144}");
    assert!((b144.distance_m - b60.distance_m).abs() <= 0.02 * b60.distance_m);
    assert!(near(b144.distance_m, 41.11, "m"));
    let lead = fast.lead(V100).expect("reaches");
    assert!((lead - 5.50).abs() <= 0.05, "T_LEAD at 144 Hz: {lead}");
    let slide = metrics::slide(&fast);
    assert!(slide.breakaway.is_some_and(|rung| (rung - 0.75).abs() < 1e-9), "{:?}", slide.breakaway);
    assert!(slide.recover_s.is_some_and(|seconds| (seconds - 0.60).abs() <= 0.05), "{:?}", slide.recover_s);
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
    assert!(near(skid_low, 0.815, "g") && near(skid_high, 1.489, "g"));
    assert!(near(brake_low, 0.879, "g") && near(brake_high, 1.228, "g"));
    assert!(skid_high > skid_low && skid_high < 2.0 * skid_low);
    assert!(brake_high > brake_low && brake_high < 2.0 * brake_low);
}
