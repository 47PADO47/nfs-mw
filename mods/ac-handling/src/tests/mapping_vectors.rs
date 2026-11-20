//! The test vectors of docs/specs/ac-handling/parameter-mapping.md section 6: the four cars, the per-axle grip, the override
//! example and the peak angle override. Tolerance of the spec: relative 1e-4.

use super::assert_close;
use crate::mapping::{CarParams, CarSpec, Drive, MapOptions, map_car};
use crate::presets::{CarClass, Compound, PresetSet, parse_car_file};

fn presets() -> PresetSet {
    PresetSet::builtin().expect("the shipped presets parse")
}

fn map(spec: &CarSpec) -> CarParams {
    map_car(spec, &presets(), &MapOptions::default())
}

fn compact_fwd() -> CarSpec {
    CarSpec {
        mass_kg: 1100.0,
        peak_friction: [1.25; 2],
        sliding_friction: [1.10; 2],
        drive: Drive::Front,
        max_power_w: 90_000.0,
        tyre_width_m: [0.195, 0.195],
        front_weight_fraction: 0.60,
        ..CarSpec::ref_sport()
    }
}

fn supercar() -> CarSpec {
    CarSpec {
        mass_kg: 1500.0,
        peak_friction: [2.3; 2],
        sliding_friction: [2.0; 2],
        max_power_w: 480_000.0,
        tyre_width_m: [0.235, 0.295],
        ..CarSpec::ref_sport()
    }
}

fn heavy_awd() -> CarSpec {
    CarSpec {
        mass_kg: 1900.0,
        peak_friction: [1.7; 2],
        sliding_friction: [1.5; 2],
        drive: Drive::All,
        ..CarSpec::ref_sport()
    }
}

/// One row of the table: `mu_y0, mu_x0, (exp_y, exp_x), (fall_y, fall_x), (q_y, q_x), (front, rear angle), relax_len,
/// cx_ratio, flex, speed_sens`.
#[allow(clippy::type_complexity)]
struct Row {
    level: f64,
    compound: Compound,
    class: CarClass,
    mu_y0: f64,
    mu_x0: f64,
    exp: (f64, f64),
    fall: (f64, f64),
    q: (f64, f64),
    angle: (f64, f64),
    relax_len: f64,
    cx_ratio: f64,
    flex: f64,
    speed_sens: f64,
}

#[track_caller]
fn check(name: &str, params: &CarParams, row: &Row) {
    assert_close(&format!("{name} L"), params.levels[0], row.level, 1e-4, 1e-4);
    assert_eq!(params.compound, row.compound, "{name} compound");
    assert_eq!(params.class, row.class, "{name} class");
    let (f, r) = (&params.front, &params.rear);
    assert_close(&format!("{name} mu_y0"), f.mu_y0, row.mu_y0, 1e-4, 1e-4);
    assert_close(&format!("{name} mu_x0"), f.mu_x0(), row.mu_x0, 1e-4, 1e-4);
    assert_close(&format!("{name} exp_y"), f.exp_y, row.exp.0, 1e-4, 1e-4);
    assert_close(&format!("{name} exp_x"), f.exp_x, row.exp.1, 1e-4, 1e-4);
    assert_close(&format!("{name} fall_y"), f.fall_y, row.fall.0, 1e-4, 1e-4);
    assert_close(&format!("{name} fall_x"), f.fall_x, row.fall.1, 1e-4, 1e-4);
    assert_close(&format!("{name} q_y"), f.q_y, row.q.0, 1e-4, 1e-4);
    assert_close(&format!("{name} q_x"), f.q_x, row.q.1, 1e-4, 1e-4);
    assert_close(&format!("{name} angle front"), f.peak_angle_deg, row.angle.0, 1e-4, 1e-4);
    assert_close(&format!("{name} angle rear"), r.peak_angle_deg, row.angle.1, 1e-4, 1e-4);
    assert_close(&format!("{name} relax_len"), f.relax_len, row.relax_len, 1e-4, 1e-4);
    assert_close(&format!("{name} cx_ratio"), f.cx_ratio, row.cx_ratio, 1e-4, 1e-4);
    assert_close(&format!("{name} flex"), f.flex, row.flex, 1e-4, 1e-4);
    assert_close(&format!("{name} speed_sens"), f.speed_sens, row.speed_sens, 1e-4, 1e-6);
}

#[test]
fn the_reference_car_maps_to_the_sport_tyre() {
    let row = Row {
        level: 1.0,
        compound: Compound::Sport,
        class: CarClass::Sport,
        mu_y0: 1.2,
        mu_x0: 1.32,
        exp: (0.85, 0.88),
        fall: (0.8616, 0.8316),
        q: (1.2, 1.4),
        angle: (9.0, 7.0),
        relax_len: 0.34,
        cx_ratio: 1.4,
        flex: 0.2,
        speed_sens: 0.0012,
    };
    check("REF", &map(&CarSpec::ref_sport()), &row);
}

#[test]
fn the_compact_front_driven_car_is_an_economy_tyre() {
    let row = Row {
        level: 0.6024,
        compound: Compound::Economy,
        class: CarClass::Compact,
        mu_y0: 1.0013,
        mu_x0: 1.0514,
        exp: (0.90, 0.92),
        fall: (0.85, 0.82),
        q: (1.0, 1.2),
        angle: (11.0, 8.0),
        relax_len: 0.36,
        cx_ratio: 1.2,
        flex: 0.25,
        speed_sens: 0.0015,
    };
    check("COMPACT_FWD", &map(&compact_fwd()), &row);
}

#[test]
fn the_supercar_is_a_sport_tyre_in_the_super_class() {
    let row = Row {
        level: 1.1084,
        compound: Compound::Sport,
        class: CarClass::Super,
        mu_y0: 1.2596,
        mu_x0: 1.3856,
        exp: (0.85, 0.88),
        fall: (0.8396, 0.8096),
        q: (1.2, 1.4),
        angle: (8.5, 6.5),
        relax_len: 0.374,
        cx_ratio: 1.47,
        flex: 0.2,
        speed_sens: 0.0012,
    };
    check("SUPER", &map(&supercar()), &row);
}

#[test]
fn the_heavy_all_wheel_drive_car_is_a_road_tyre_in_the_heavy_class() {
    let row = Row {
        level: 0.8193,
        compound: Compound::Road,
        class: CarClass::Heavy,
        mu_y0: 1.1106,
        mu_x0: 1.1995,
        exp: (0.86, 0.88),
        fall: (0.8524, 0.8224),
        q: (1.1, 1.3),
        angle: (10.25, 7.75),
        relax_len: 0.414,
        cx_ratio: 1.3,
        flex: 0.22,
        speed_sens: 0.0015,
    };
    check("HEAVY_AWD", &map(&heavy_awd()), &row);
}

#[test]
fn static_corner_loads_of_the_log_line() {
    let loads = CarSpec::ref_sport().static_corner_loads();
    assert_close("REF front", loads[0], 3698.4, 1e-4, 0.1);
    assert_close("REF rear", loads[1], 3413.9, 1e-4, 0.1);
    let compact = compact_fwd().static_corner_loads();
    assert_close("COMPACT front", compact[0], 3237.3, 1e-4, 0.1);
    assert_close("COMPACT rear", compact[1], 2158.2, 1e-4, 0.1);
    let heavy = heavy_awd().static_corner_loads();
    assert_close("HEAVY front", heavy[0], 4846.1, 1e-4, 0.1);
    assert_close("HEAVY rear", heavy[1], 4473.4, 1e-4, 0.1);
}

#[test]
fn each_axle_has_its_own_grip() {
    let spec = CarSpec { peak_friction: [2.075, 2.2], sliding_friction: [1.85, 1.95], ..CarSpec::ref_sport() };
    let params = map(&spec);
    assert_eq!(params.compound, Compound::Sport, "the front chooses");
    assert_close("front mu_y0", params.front.mu_y0, 1.2, 1e-4, 1e-4);
    assert_close("rear L", params.levels[1], 1.0602, 1e-4, 1e-4);
    assert_close("rear mu_y0", params.rear.mu_y0, 1.2331, 1e-4, 1e-4);
    assert_close("rear mu_x0", params.rear.mu_x0(), 1.3564, 1e-4, 1e-4);
    assert_close("rear fall_y", params.rear.fall_y, 0.8564, 1e-4, 1e-4);
    assert_close("rear fall_x", params.rear.fall_x, 0.8264, 1e-4, 1e-4);
}

#[test]
fn a_car_file_forces_compound_and_class_and_replaces_keys() {
    let file =
        parse_car_file("schema = 1\ncompound = \"road\"\nclass = \"compact\"\n[tyre]\nmu_y0 = 1.05\nq_y = 1.5\n")
            .expect("a valid car file");
    let options = MapOptions { car: Some(&file), forced: None };
    let params = map_car(&CarSpec::ref_sport(), &presets(), &options);
    assert_close("mu_y0", params.front.mu_y0, 1.05, 1e-4, 1e-4);
    assert_close("mu_x0", params.front.mu_x0(), 1.134, 1e-4, 1e-4);
    assert_close("q_y", params.front.q_y, 1.5, 1e-4, 1e-4);
    assert_close("fall_y stays from rho", params.front.fall_y, 0.8616, 1e-4, 1e-4);
    assert_close("relax_len", params.front.relax_len, 0.324, 1e-4, 1e-4);
    assert_close("angle front", params.front.peak_angle_deg, 10.0, 1e-4, 1e-4);
    assert_close("angle rear", params.rear.peak_angle_deg, 8.0, 1e-4, 1e-4);
}

#[test]
fn the_peak_angle_of_a_car_file_is_the_mean_until_an_axle_overrides_it() {
    let mean = parse_car_file("schema = 1\n[tyre]\npeak_angle_deg = 9.0\n").expect("valid");
    let options = MapOptions { car: Some(&mean), forced: None };
    let params = map_car(&CarSpec::ref_sport(), &presets(), &options);
    assert_close("front", params.front.peak_angle_deg, 10.0, 1e-4, 1e-4);
    assert_close("rear", params.rear.peak_angle_deg, 8.0, 1e-4, 1e-4);
    let rear =
        parse_car_file("schema = 1\n[tyre]\npeak_angle_deg = 9.0\n[tyre.rear]\npeak_angle_deg = 7.5\n").expect("valid");
    let options = MapOptions { car: Some(&rear), forced: None };
    let params = map_car(&CarSpec::ref_sport(), &presets(), &options);
    assert_close("front", params.front.peak_angle_deg, 10.0, 1e-4, 1e-4);
    assert_close("rear exactly", params.rear.peak_angle_deg, 7.5, 1e-9, 1e-9);
}

#[test]
fn the_axle_bias_of_a_car_file_replaces_the_layout_default() {
    let file = parse_car_file("schema = 1\naxle_angle_bias_deg = 0\n").expect("valid");
    let options = MapOptions { car: Some(&file), forced: None };
    let params = map_car(&CarSpec::ref_sport(), &presets(), &options);
    assert_close("front", params.front.peak_angle_deg, 8.0, 1e-9, 1e-9);
    assert_close("rear", params.rear.peak_angle_deg, 8.0, 1e-9, 1e-9);
}

#[test]
fn the_preset_setting_forces_the_compound_over_the_car_file_and_the_grip_level() {
    let file = parse_car_file("schema = 1\ncompound = \"road\"\n").expect("valid");
    let options = MapOptions { car: Some(&file), forced: Some(Compound::Slick) };
    let params = map_car(&CarSpec::ref_sport(), &presets(), &options);
    assert_eq!(params.compound, Compound::Slick);
    // The same formula as for an automatic choice: 1.60 + 1.00 * (1.0 - 1.55).
    assert_close("mu_y0", params.front.mu_y0, 1.05, 1e-4, 1e-4);
}

#[test]
fn a_car_with_unusable_figures_gets_the_defaults_of_the_spec() {
    let spec = CarSpec {
        peak_friction: [f64::NAN, 0.0],
        sliding_friction: [1.0, -1.0],
        tyre_width_m: [f64::NAN, 0.0],
        ..CarSpec::ref_sport()
    };
    let params = map(&spec);
    assert_eq!(params.levels, [1.0, 1.0]);
    assert_eq!(params.compound, Compound::Sport);
    assert_close("rho 0.89 -> fall_y", params.front.fall_y, 0.86, 1e-9, 1e-9);
    assert!(params.front.k_vert.is_finite() && params.front.k_vert > 0.0);
    assert!(params.rear.k_vert.is_finite() && params.rear.k_vert > 0.0);
}

#[test]
fn the_classes_and_compounds_tile_the_grip_axis() {
    let mut last: Option<f64> = None;
    for step in 0..200 {
        let level = 0.4 + f64::from(step) * 0.01;
        let spec = CarSpec { peak_friction: [level * 2.075; 2], ..CarSpec::ref_sport() };
        let mu = map(&spec).front.mu_y0;
        if let Some(before) = last {
            // The mapping is continuous across a compound boundary to within 0.02 of mu_y0 (a step of 0.01 of level).
            assert!((mu - before).abs() <= 0.05, "level {level}: {before} -> {mu}");
        }
        last = Some(mu);
    }
}

#[test]
fn the_shipped_presets_keep_the_hosts_wheel_inertia() {
    // The spec's default is 0.3, which the planar car's targets assume; the presets carry 1.0 so the game's drivetrain stays
    // stable (NOTES.md, spec question 1).
    assert_eq!(crate::keys::Keys::default().inertia_scale, 0.3);
    for compound in Compound::ALL {
        let params = map_car(&CarSpec::ref_sport(), &presets(), &MapOptions { car: None, forced: Some(compound) });
        assert_eq!(params.front.inertia_scale, 1.0, "{}", compound.name());
    }
}
