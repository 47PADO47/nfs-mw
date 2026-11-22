//! The revision of parameter-mapping.md (2026-11-20): the forced compound re-bases the grip level, the precedence of the
//! compound, the car id, the default `inertia_scale`, and the clamp of step 9.

use super::assert_close;
use crate::keys::Keys;
use crate::mapping::{CarSpec, MapOptions, map_car};
use crate::presets::{Compound, PresetSet, parse_car_file};

fn presets() -> PresetSet {
    PresetSet::builtin().expect("the shipped presets parse")
}

fn forced(spec: &CarSpec, compound: Compound) -> crate::mapping::CarParams {
    map_car(spec, &presets(), &MapOptions { car: None, forced: Some(compound) })
}

fn rear_2_2() -> CarSpec {
    CarSpec { peak_friction: [2.075, 2.2], sliding_friction: [1.85, 1.95], ..CarSpec::ref_sport() }
}

fn compact() -> CarSpec {
    CarSpec {
        mass_kg: 1100.0,
        peak_friction: [1.25; 2],
        sliding_friction: [1.10; 2],
        max_power_w: 90_000.0,
        tyre_width_m: [0.195, 0.195],
        front_weight_fraction: 0.60,
        ..CarSpec::ref_sport()
    }
}

fn heavy() -> CarSpec {
    CarSpec { mass_kg: 1900.0, peak_friction: [1.7; 2], sliding_friction: [1.5; 2], ..CarSpec::ref_sport() }
}

#[test]
fn the_forced_compound_table_of_the_spec() {
    // Rows: economy, road, sport, semislick, slick. Columns: ref_sport front (= rear), the rear-2.2 car front and rear,
    // COMPACT_FWD, HEAVY_AWD (the layout of those two does not matter for mu_y0).
    let table = [
        (Compound::Economy, 1.0000, (1.0000, 1.0331), 1.0000, 1.0000),
        (Compound::Road, 1.1000, (1.1000, 1.1331), 1.1000, 1.1000),
        (Compound::Sport, 1.2000, (1.2000, 1.2331), 1.2000, 1.2000),
        (Compound::Semislick, 1.3500, (1.3500, 1.3982), 1.3500, 1.3500),
        (Compound::Slick, 1.6000, (1.6000, 1.6602), 1.6000, 1.6000),
    ];
    for (compound, reference, (front, rear), compact_mu, heavy_mu) in table {
        let name = compound.name();
        assert_close(
            &format!("{name} ref"),
            forced(&CarSpec::ref_sport(), compound).front.mu_y0,
            reference,
            1e-4,
            1e-4,
        );
        let axles = forced(&rear_2_2(), compound);
        assert_close(&format!("{name} front"), axles.front.mu_y0, front, 1e-4, 1e-4);
        assert_close(&format!("{name} rear"), axles.rear.mu_y0, rear, 1e-4, 1e-4);
        assert_close(&format!("{name} compact"), forced(&compact(), compound).front.mu_y0, compact_mu, 1e-4, 1e-4);
        assert_close(&format!("{name} heavy"), forced(&heavy(), compound).front.mu_y0, heavy_mu, 1e-4, 1e-4);
    }
}

#[test]
fn a_forced_slick_is_never_below_a_forced_sport() {
    for spec in [CarSpec::ref_sport(), compact(), heavy(), rear_2_2()] {
        let slick = forced(&spec, Compound::Slick).front.mu_y0;
        let sport = forced(&spec, Compound::Sport).front.mu_y0;
        assert!(slick > sport, "{slick} against {sport}");
    }
}

#[test]
fn forcing_a_compound_equals_the_automatic_mapping_of_the_re_levelled_car() {
    // The invariant of the spec: the front peak friction becomes level_ref * 2.075, the rear shifts by the same difference, and
    // both sliding frictions are scaled to keep each axle's rho.
    let set = presets();
    for spec in [CarSpec::ref_sport(), compact(), heavy(), rear_2_2()] {
        for compound in Compound::ALL {
            let level_ref = set.compound(compound).level_ref;
            let (l_front, l_rear) = (spec.peak_friction[0] / 2.075, spec.peak_friction[1] / 2.075);
            let peak = [level_ref * 2.075, (level_ref + (l_rear - l_front)) * 2.075];
            let rho =
                [spec.sliding_friction[0] / spec.peak_friction[0], spec.sliding_friction[1] / spec.peak_friction[1]];
            let moved =
                CarSpec { peak_friction: peak, sliding_friction: [peak[0] * rho[0], peak[1] * rho[1]], ..spec.clone() };
            let by_force = forced(&spec, compound);
            let by_level = map_car(&moved, &set, &MapOptions::default());
            assert_eq!(by_level.compound, compound, "the reference levels fall inside their own class");
            for (forced_axle, level_axle) in [(&by_force.front, &by_level.front), (&by_force.rear, &by_level.rear)] {
                for key in Keys::NAMES {
                    let (a, b) = (forced_axle.get(key).expect("a key"), level_axle.get(key).expect("a key"));
                    assert_close(&format!("{} {key}", compound.name()), a, b, 1e-9, 1e-9);
                }
            }
        }
    }
}

#[test]
fn the_compound_precedence_is_setting_then_car_file_then_automatic() {
    let set = presets();
    let file = parse_car_file("schema = 1\ncompound = \"road\"\n").expect("valid");
    let spec = CarSpec::ref_sport();
    let both = map_car(&spec, &set, &MapOptions { car: Some(&file), forced: Some(Compound::Slick) });
    assert_eq!(both.compound, Compound::Slick, "the setting wins");
    let file_only = map_car(&spec, &set, &MapOptions { car: Some(&file), forced: None });
    assert_eq!(file_only.compound, Compound::Road, "with preset = auto the file's compound applies");
    let neither = map_car(&spec, &set, &MapOptions::default());
    assert_eq!(neither.compound, Compound::Sport, "with neither, the car's own grip picks sport");
}

#[test]
fn the_car_id_rounds_to_the_nearest_integer_and_rejects_unusable_numbers() {
    assert_eq!(CarSpec::ref_sport().key_name(), "spec-1450-245-300");
    let ids = [
        (CarSpec { mass_kg: 1100.0, max_power_w: 90_000.0, ..CarSpec::ref_sport() }, "spec-1100-245-90"),
        (CarSpec { mass_kg: 1500.0, max_power_w: 480_000.0, ..CarSpec::ref_sport() }, "spec-1500-245-480"),
        (CarSpec { mass_kg: 1900.0, ..CarSpec::ref_sport() }, "spec-1900-245-300"),
        (CarSpec { max_power_w: 0.0, ..CarSpec::ref_sport() }, "spec-1450-245-0"),
        (CarSpec { mass_kg: 1449.5, ..CarSpec::ref_sport() }, "spec-1450-245-300"),
        (CarSpec { mass_kg: 1449.49, ..CarSpec::ref_sport() }, "spec-1449-245-300"),
        (CarSpec { wheelbase_m: f64::from(2.455_f32), ..CarSpec::ref_sport() }, "spec-1450-245-300"),
        (CarSpec { max_power_w: 299_500.0, ..CarSpec::ref_sport() }, "spec-1450-245-300"),
        (CarSpec { max_power_w: 299_499.0, ..CarSpec::ref_sport() }, "spec-1450-245-299"),
    ];
    for (spec, id) in ids {
        assert_eq!(spec.key_name(), id);
    }
    // 2.45f32 * 100 is 245.000005 in the widened arithmetic.
    assert_eq!(CarSpec { wheelbase_m: f64::from(2.45_f32), ..CarSpec::ref_sport() }.key_name(), "spec-1450-245-300");
    let bad = [
        CarSpec { mass_kg: f64::NAN, ..CarSpec::ref_sport() },
        CarSpec { mass_kg: 0.0, ..CarSpec::ref_sport() },
        CarSpec { wheelbase_m: -1.0, ..CarSpec::ref_sport() },
        CarSpec { max_power_w: -5.0, ..CarSpec::ref_sport() },
        CarSpec { max_power_w: f64::INFINITY, ..CarSpec::ref_sport() },
    ];
    for spec in bad {
        assert_eq!(spec.key_name(), "spec-invalid");
    }
}

#[test]
fn the_default_inertia_scale_is_one_and_the_loader_accepts_the_light_wheel() {
    assert_eq!(Keys::default().inertia_scale, 1.0);
    let params = map_car(&CarSpec::ref_sport(), &presets(), &MapOptions::default());
    assert_eq!((params.front.inertia_scale, params.rear.inertia_scale), (1.0, 1.0));
    let light = parse_car_file("schema = 1\n[tyre]\ninertia_scale = 0.3\n").expect("0.3 is inside the range");
    let light = map_car(&CarSpec::ref_sport(), &presets(), &MapOptions { car: Some(&light), forced: None });
    assert_eq!(light.front.inertia_scale, 0.3);
}

#[test]
fn step_9_clamps_a_combination_that_leaves_a_range() {
    // A mean peak angle of 14 with a bias of 8 gives a front 18, clamped to 14; the rear is 10.
    let file = parse_car_file("schema = 1\naxle_angle_bias_deg = 8\n[tyre]\npeak_angle_deg = 14\n").expect("valid");
    let params = map_car(&CarSpec::ref_sport(), &presets(), &MapOptions { car: Some(&file), forced: None });
    assert_close("front", params.front.peak_angle_deg, 14.0, 1e-9, 1e-9);
    assert_close("rear", params.rear.peak_angle_deg, 10.0, 1e-9, 1e-9);
}
