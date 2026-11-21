//! The loaders: the shipped presets, and one test per class of TOML mistake (parameter-mapping.md sections 5.4 and 6).
//! A file with a mistake is rejected whole with a one-line reason; the layer falls back to the built-in copy.

use super::TempDir;
use crate::keys::Keys;
use crate::presets::{CarClass, Compound, PresetSet, parse_car_file, parse_preset};
use crate::settings::{PresetChoice, parse_config};

const SPORT: &str =
    "schema = 1\nkind = \"compound\"\nname = \"sport\"\nlevel_ref = 1.0\ngrip_slope = 0.55\n[tyre]\nmu_y0 = 1.2\n";
const SUPER: &str =
    "schema = 1\nkind = \"class\"\nname = \"super\"\n[mul]\nrelax_len = 1.1\n[add]\npeak_angle_deg = -0.5\n";

fn compound_error(text: &str) -> String {
    parse_preset("compound-sport.toml", text).expect_err("the file must be rejected")
}

fn class_error(text: &str) -> String {
    parse_preset("class-super.toml", text).expect_err("the file must be rejected")
}

#[test]
fn the_shipped_files_parse_and_cover_every_compound_and_class() {
    let set = PresetSet::builtin().expect("the shipped presets");
    for compound in Compound::ALL {
        assert_eq!(set.compound(compound).compound, compound);
    }
    for class in CarClass::ALL {
        assert_eq!(set.class(class).class, class);
    }
    assert_eq!(set.compound(Compound::Sport).tyre.mu_y0, Some(1.20));
    assert_eq!(set.compound(Compound::Slick).level_ref, 1.55);
    assert_eq!(set.class(CarClass::Heavy).add.exp_y, Some(-0.02));
}

#[test]
fn the_table_of_5_2_is_what_the_files_hold() {
    // Every row of parameter-mapping.md 5.2 for the five compounds: level_ref, grip_slope, mu_y0, mu_x_ratio, exp_y, exp_x,
    // peak_angle_deg, flex, cx_ratio, q_y, q_x, speed_sens, relax_len.
    let rows: [(Compound, [f64; 13]); 5] = [
        (Compound::Economy, [0.60, 0.55, 1.00, 1.05, 0.90, 0.92, 9.0, 0.25, 1.2, 1.0, 1.2, 0.0015, 0.40]),
        (Compound::Road, [0.80, 0.55, 1.10, 1.08, 0.88, 0.90, 8.5, 0.22, 1.3, 1.1, 1.3, 0.0015, 0.36]),
        (Compound::Sport, [1.00, 0.55, 1.20, 1.10, 0.85, 0.88, 8.0, 0.20, 1.4, 1.2, 1.4, 0.0012, 0.34]),
        (Compound::Semislick, [1.25, 0.80, 1.35, 1.12, 0.82, 0.86, 7.5, 0.18, 1.6, 1.5, 1.8, 0.0010, 0.30]),
        (Compound::Slick, [1.55, 1.00, 1.60, 1.12, 0.80, 0.84, 7.0, 0.15, 1.8, 2.0, 2.4, 0.0008, 0.26]),
    ];
    let set = PresetSet::builtin().expect("the shipped presets");
    for (compound, want) in rows {
        let preset = set.compound(compound);
        let t = &preset.tyre;
        let got = [
            preset.level_ref,
            preset.grip_slope,
            t.mu_y0.expect("mu_y0"),
            t.mu_x_ratio.expect("mu_x_ratio"),
            t.exp_y.expect("exp_y"),
            t.exp_x.expect("exp_x"),
            t.peak_angle_deg.expect("peak_angle_deg"),
            t.flex.expect("flex"),
            t.cx_ratio.expect("cx_ratio"),
            t.q_y.expect("q_y"),
            t.q_x.expect("q_x"),
            t.speed_sens.expect("speed_sens"),
            t.relax_len.expect("relax_len"),
        ];
        assert_eq!(got, want, "{}", compound.name());
    }
    // The informational fall values of the table.
    let fall = [(Compound::Economy, 0.88, 0.85), (Compound::Road, 0.87, 0.84), (Compound::Slick, 0.78, 0.74)];
    for (compound, y, x) in fall {
        let t = &set.compound(compound).tyre;
        assert_eq!((t.fall_y, t.fall_x), (Some(y), Some(x)), "{}", compound.name());
    }
}

#[test]
fn an_empty_presets_folder_gives_the_built_in_set() {
    let dir = TempDir::new("empty-presets");
    let (set, notes) = PresetSet::load(&dir.0).expect("loads");
    assert!(notes.is_empty());
    assert_eq!(set, PresetSet::builtin().expect("the shipped presets"));
    let (missing, _) = PresetSet::load(&dir.0.join("not-there")).expect("a missing folder is the same");
    assert_eq!(missing, set);
}

#[test]
fn a_valid_file_in_the_folder_replaces_the_built_in_one_and_a_broken_one_does_not() {
    let dir = TempDir::new("override-presets");
    dir.write("compound-sport.toml", &SPORT.replace("mu_y0 = 1.2", "mu_y0 = 1.25"));
    dir.write("class-super.toml", "schema = 1\nkind = \"class\"\nname = \"super\"\n[tyre]\nmu_y0 = 1.0\n");
    let (set, notes) = PresetSet::load(&dir.0).expect("loads");
    assert_eq!(set.compound(Compound::Sport).tyre.mu_y0, Some(1.25));
    let builtin = PresetSet::builtin().expect("the shipped presets");
    assert_eq!(set.class(CarClass::Super), builtin.class(CarClass::Super), "the broken file fell back");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("class-super.toml") && notes[0].contains("built-in"), "{}", notes[0]);
}

#[test]
fn loader_mistake_an_unknown_key_is_rejected() {
    assert!(compound_error(&SPORT.replace("[tyre]", "[tyre]\nmystery = 1.0")).contains("mystery"));
    assert!(compound_error(&format!("{SPORT}extra = 1\n")).contains("extra"));
    assert!(class_error(&SUPER.replace("[add]", "[add]\nmystery = 1.0")).contains("mystery"));
    assert!(parse_car_file("schema = 1\n[tyre]\nmystery = 1.0\n").is_err());
    assert!(parse_car_file("schema = 1\n[tyre.front]\nmystery = 1.0\n").is_err());
    assert!(parse_car_file("schema = 1\nmystery = 1\n").is_err());
}

#[test]
fn loader_mistake_a_number_out_of_range_is_rejected() {
    assert!(compound_error(&SPORT.replace("mu_y0 = 1.2", "mu_y0 = 9.0")).contains("mu_y0"));
    assert!(compound_error(&SPORT.replace("mu_y0 = 1.2", "mu_y0 = 0.1")).contains("mu_y0"));
    assert!(compound_error(&SPORT.replace("level_ref = 1.0", "level_ref = 9.0")).contains("level_ref"));
    assert!(class_error(&SUPER.replace("relax_len = 1.1", "relax_len = 99.0")).contains("relax_len"));
    assert!(parse_car_file("schema = 1\n[tyre]\nexp_y = 1.5\n").expect_err("range").contains("exp_y"));
    assert!(parse_car_file("schema = 1\n[tyre.rear]\nq_x = 0.0\n").expect_err("range").contains("q_x"));
    assert!(
        parse_car_file("schema = 1\naxle_angle_bias_deg = 12\n").expect_err("range").contains("axle_angle_bias_deg")
    );
    // Every key has a range, and the defaults are inside it.
    let defaults = Keys::default();
    for key in Keys::NAMES {
        let (lo, hi) = Keys::range(key).expect("a range");
        let value = defaults.get(key).expect("a default");
        assert!((lo..=hi).contains(&value), "default of {key}");
    }
}

#[test]
fn loader_mistake_a_number_that_is_not_finite_is_rejected() {
    assert!(compound_error(&SPORT.replace("mu_y0 = 1.2", "mu_y0 = nan")).contains("finite"));
    assert!(compound_error(&SPORT.replace("mu_y0 = 1.2", "mu_y0 = inf")).contains("finite"));
    assert!(parse_car_file("schema = 1\n[tyre]\nmu_y0 = nan\n").expect_err("nan").contains("finite"));
    assert!(parse_config("grip_gain = nan\n").is_err());
}

#[test]
fn loader_mistake_a_missing_or_wrong_schema_is_rejected() {
    assert!(compound_error(&SPORT.replace("schema = 1\n", "")).contains("schema"));
    assert!(compound_error(&SPORT.replace("schema = 1", "schema = 2")).contains("schema"));
    assert!(parse_car_file("[tyre]\nmu_y0 = 1.1\n").expect_err("missing").contains("schema"));
    assert!(parse_car_file("schema = 7\n").expect_err("wrong").contains("schema"));
}

#[test]
fn loader_mistake_a_name_that_differs_from_the_file_name_is_rejected() {
    let reason = parse_preset("compound-road.toml", SPORT).expect_err("name");
    assert!(reason.contains("compound-sport") && reason.contains("compound-road.toml"), "{reason}");
    assert!(parse_preset("class-sport.toml", SUPER).is_err());
    assert!(parse_preset("sport.toml", SPORT).is_err(), "the kind prefix is part of the file name");
    // A kind that does not match its prefix is the same mistake.
    assert!(parse_preset("compound-super.toml", SUPER).is_err());
}

#[test]
fn loader_mistake_a_class_with_a_tyre_table_or_a_compound_with_factors_is_rejected() {
    let reason = class_error(&SUPER.replace("[mul]", "[tyre]\nmu_y0 = 1.0\n[mul]"));
    assert!(reason.contains("[tyre]"), "{reason}");
    let with_level = SUPER.replace(
        "name = \"super\"",
        "name = \"super\"
level_ref = 1.0",
    );
    assert!(class_error(&with_level).contains("level_ref"));
    assert!(compound_error(&format!("{SPORT}[mul]\nrelax_len = 1.1\n")).contains("[mul]"));
    assert!(compound_error(&format!("{SPORT}[add]\nrelax_len = 0.1\n")).contains("[add]"));
}

#[test]
fn loader_mistake_a_compound_without_its_grip_keys_or_a_class_that_is_not_one() {
    assert!(compound_error(&SPORT.replace("level_ref = 1.0\n", "")).contains("level_ref"));
    assert!(compound_error(&SPORT.replace("grip_slope = 0.55\n", "")).contains("grip_slope"));
    assert!(parse_preset("compound-gold.toml", &SPORT.replace("sport", "gold")).is_err());
    assert!(parse_preset("class-tiny.toml", &SUPER.replace("super", "tiny")).is_err());
    assert!(parse_preset("thing-sport.toml", &SPORT.replace("compound", "thing")).is_err());
}

#[test]
fn loader_mistake_a_factor_out_of_bounds_or_on_the_axle_bias_is_rejected() {
    assert!(class_error(&SUPER.replace("relax_len = 1.1", "relax_len = 0.0")).contains("mul.relax_len"));
    assert!(class_error(&SUPER.replace("relax_len = 1.1", "relax_len = 50.0")).contains("mul.relax_len"));
    assert!(
        class_error(&SUPER.replace("peak_angle_deg = -0.5", "peak_angle_deg = 500.0")).contains("add.peak_angle_deg")
    );
    assert!(class_error(&SUPER.replace("[add]", "[add]\naxle_angle_bias_deg = 1.0")).contains("axle_angle_bias_deg"));
}

#[test]
fn loader_mistake_a_value_of_the_wrong_type_is_rejected() {
    assert!(!compound_error(&SPORT.replace("mu_y0 = 1.2", "mu_y0 = \"high\"")).is_empty());
    assert!(parse_car_file("schema = 1\ncompound = 3\n").is_err());
    assert!(parse_car_file("schema = 1\n[tyre]\nmu_y0 = [1.0]\n").is_err());
    assert!(parse_config("substeps = \"ten\"\n").is_err());
    assert!(!compound_error("this is not toml").is_empty());
}

#[test]
fn loader_mistake_the_axle_bias_belongs_at_the_top_of_a_car_file() {
    let reason = parse_car_file("schema = 1\n[tyre]\naxle_angle_bias_deg = 2.0\n").expect_err("bias in [tyre]");
    assert!(reason.contains("top level"), "{reason}");
    let reason = parse_car_file("schema = 1\n[tyre.front]\naxle_angle_bias_deg = 2.0\n").expect_err("bias in front");
    assert!(reason.contains("[tyre.front]"), "{reason}");
    assert!(parse_car_file("schema = 1\ncompound = \"gold\"\n").is_err());
    assert!(parse_car_file("schema = 1\nclass = \"tiny\"\n").is_err());
}

#[test]
fn a_car_file_may_be_empty_and_integers_are_numbers() {
    let file = parse_car_file("schema = 1\n").expect("an empty car file is valid");
    assert_eq!(file, crate::presets::CarFile::default());
    let file = parse_car_file("schema = 1\n[tyre]\nmu_y0 = 1\n").expect("an integer is a number");
    assert_eq!(file.both.mu_y0, Some(1.0));
    let file = parse_car_file("schema = 1\ncompound = \"slick\"\nclass = \"heavy\"\naxle_angle_bias_deg = 1.5\n")
        .expect("valid");
    assert_eq!((file.compound, file.class, file.bias), (Some(Compound::Slick), Some(CarClass::Heavy), Some(1.5)));
}

#[test]
fn the_config_file_gives_the_first_values_and_rejects_mistakes_whole() {
    let all = parse_config("enabled = true\npreset = \"road\"\ngrip_gain = 1.2\nsubsteps = 6\n").expect("valid");
    assert!(all.enabled);
    assert_eq!(all.preset, PresetChoice::Forced(Compound::Road));
    assert_eq!((all.grip_gain, all.substeps), (1.2, 6));
    let empty = parse_config("").expect("an empty config is the defaults");
    assert!(!empty.enabled);
    assert_eq!((empty.preset, empty.grip_gain, empty.substeps), (PresetChoice::Auto, 1.0, 0));
    assert!(parse_config("mystery = 1").is_err());
    assert!(parse_config("grip_gain = 3.0").is_err());
    assert!(parse_config("grip_gain = 0.1").is_err());
    assert!(parse_config("substeps = 25").is_err());
    assert!(parse_config("preset = \"gold\"").is_err());
}

#[test]
fn the_units_suspension_and_steering_tables_of_a_tuning_file() {
    let file = parse_car_file(
        "schema = 1\n[units]\naids = true\nsteering = true\ntc = 2\nabs = 3\nesc = 1\nbrake_bias_clicks = -3\n\
         [suspension.front]\nk_w = 35000\nh_rc = 0.08\n[suspension.rear]\nk_arb = 9000\n[steering]\ndelta_max_deg = 35\ntheta_max_deg = 540\n",
    )
    .expect("valid");
    assert_eq!((file.units.aids, file.units.steering, file.units.tc), (Some(true), Some(true), Some(2)));
    assert_eq!(file.susp_front.k_w, Some(35000.0));
    assert_eq!(file.susp_rear.k_arb, Some(9000.0));
    assert_eq!(file.steering.delta_max_deg, Some(35.0));
    let mapped = crate::mapping::map_car(
        &crate::mapping::CarSpec::ref_sport(),
        &PresetSet::builtin().expect("presets"),
        &crate::mapping::MapOptions { car: Some(&file), forced: None },
    );
    assert_eq!(mapped.suspension[0].k_w, 35000.0);
    assert_eq!(mapped.suspension[1].k_w, 28000.0, "the rear keeps its own default");
    assert_eq!(mapped.suspension[1].k_arb, 9000.0);
    assert_eq!(mapped.steering.delta_max_deg, 35.0);
    assert!(mapped.units.aids && mapped.units.steering && mapped.units.suspension, "preset default, then the file");
    assert_eq!(mapped.units.levels.tc, 2);
}

#[test]
fn loader_mistake_in_the_new_tables() {
    assert!(parse_car_file("schema = 1\n[units]\nmystery = true\n").is_err(), "an unknown unit key");
    assert!(parse_car_file("schema = 1\n[units]\ntc = 6\n").expect_err("range").contains("tc"));
    assert!(parse_car_file("schema = 1\n[units]\nesc = 5\n").is_err());
    assert!(parse_car_file("schema = 1\n[units]\nbrake_bias_clicks = 21\n").is_err());
    assert!(parse_car_file("schema = 1\n[suspension.front]\nk_w = 5\n").expect_err("range").contains("k_w"));
    assert!(parse_car_file("schema = 1\n[suspension.front]\nmystery = 1\n").is_err());
    assert!(parse_car_file("schema = 1\n[suspension.middle]\nk_w = 30000\n").is_err(), "only front and rear");
    assert!(
        parse_car_file("schema = 1\n[steering]\ndelta_max_deg = 60\n").expect_err("range").contains("delta_max_deg")
    );
    assert!(parse_car_file("schema = 1\n[steering]\nmystery = 1\n").is_err());
    assert!(parse_car_file("schema = 1\n[steering]\ntau_f = nan\n").expect_err("nan").contains("finite"));
    // A preset's [units] table is checked like a car file's; a class preset has none.
    assert!(compound_error(&format!("{SPORT}[units]\ntc = 9\n")).contains("tc"));
    assert!(compound_error(&format!("{SPORT}[units]\nmystery = 1\n")).contains("mystery"));
    assert!(class_error(&format!("{SUPER}[units]\naids = true\n")).contains("[units]"));
}

#[test]
fn every_key_set_has_defaults_inside_its_ranges() {
    use crate::keys::{SteeringKeys, SuspensionKeys};
    for key in SuspensionKeys::NAMES {
        let (lo, hi) = SuspensionKeys::range(key).expect("a range");
        for keys in [SuspensionKeys::default(), SuspensionKeys::rear()] {
            let value = keys.get(key).expect("a default");
            assert!((lo..=hi).contains(&value), "suspension {key} = {value}");
        }
    }
    for key in SteeringKeys::NAMES {
        let (lo, hi) = SteeringKeys::range(key).expect("a range");
        let value = SteeringKeys::default().get(key).expect("a default");
        assert!((lo..=hi).contains(&value), "steering {key} = {value}");
    }
    assert_eq!(Keys::default().inertia_scale, 1.0);
}

#[test]
fn the_shipped_presets_carry_their_unit_defaults() {
    let set = PresetSet::builtin().expect("presets");
    for compound in Compound::ALL {
        let units = &set.compound(compound).units;
        assert_eq!(
            (units.suspension, units.thermal, units.aids, units.steering),
            (Some(true), Some(true), Some(false), Some(false))
        );
    }
    let levels = |compound: Compound| {
        let u = &set.compound(compound).units;
        (u.tc, u.abs, u.esc)
    };
    assert_eq!(levels(Compound::Economy), (Some(3), Some(3), Some(2)));
    assert_eq!(levels(Compound::Slick), (Some(0), Some(0), Some(0)));
}
