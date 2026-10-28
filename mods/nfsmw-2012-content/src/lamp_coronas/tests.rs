use chameleon_effects::corona::pattern::Segment;
use chameleon_effects::corona::{CurveBlock, LAYER_LEN};
use chameleon_vehicle::{Locator, LocatorGroup};

use super::*;

/// A block whose lanes are all constants: `colour` of the layer's sizes and strengths do not depend on anything.
fn flat(value: f32) -> CurveBlock {
    let mut rows = [[0.0; 4]; 8];
    rows[6] = [value; 4];
    CurveBlock { rows }
}

/// A corona object whose angle lane is 1 on the axis and falls to 0.25 at the sides, and whose size and brightness lanes
/// are constants; one glow layer of constant look.
fn corona(visible_on_axis: bool, layer_colour: [f32; 4]) -> Corona {
    let mut rows = [[0.0; 4]; 8];
    rows[0] = [1.0, 0.0, 0.0, 0.0];
    rows[2] = [-0.75 / std::f32::consts::FRAC_PI_2, 0.0, 0.0, 0.0];
    rows[4] = [10.0, 10.0, 10.0, 10.0];
    rows[5] = [1.0, 0.0, 0.0, 0.0];
    rows[6] = [0.0, 1.0, 1.0, 1.0];
    rows[7] = [1.0, 0.0, 0.0, 0.0];
    if !visible_on_axis {
        rows[6] = [0.0, 1.0, 1.0, 1.0];
        rows[5] = [0.0; 4];
        rows[7] = [0.0; 4];
    }
    let glow = Layer {
        kind: LayerKind::Glow,
        curves: flat(1.0),
        colour: layer_colour,
        params: [0.0; 4],
        picture: Some(0x6D11),
        depth: 0.0,
        rotation: 0.0,
        colour_source: 0,
    };
    Corona {
        curves: CurveBlock { rows },
        far_limit: 99999.0,
        cull_distance: 100.0,
        bias: 0.0,
        beams: Vec::new(),
        flares: Vec::new(),
        glows: vec![glow],
        alt_beams: Vec::new(),
        glows_2: Vec::new(),
        glows_3: Vec::new(),
    }
}

fn record(group: u32, state_bit: u16, flash: Option<u64>) -> CoronaRecord {
    CoronaRecord {
        object: "Test_Coronas".into(),
        group,
        state_bit,
        mode: 0,
        tint: [1.0; 4],
        intensity: 1.0,
        size: [1.0; 2],
        flag: 0,
        corona: Some(7),
        flash,
    }
}

/// The 2012 vehicle space (x left, y up, z forward) to the car's (x forward, y left, z up).
fn to_car(p: [f32; 3]) -> [f32; 3] {
    [p[2], p[0], p[1]]
}

fn lamps_with(groups: &[(u32, [f32; 3], [f32; 4])]) -> Lamps {
    Lamps {
        groups: groups
            .iter()
            .map(|&(id, position, rotation)| LocatorGroup { id, locators: vec![Locator { rotation, position }] })
            .collect(),
        ..Lamps::default()
    }
}

const FRONT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const REAR: [f32; 4] = [0.0, 1.0, 0.0, 0.0];

fn no_flash(_: u64) -> Option<FlashPattern> {
    None
}

fn blink(_: u64) -> Option<FlashPattern> {
    Some(FlashPattern { segments: vec![Segment { weight: 1.0, rate: 4.0, offset: 0.0, steps: vec![1.0, 0.0] }] })
}

fn rows_of(
    records: &[CoronaRecord],
    lamps: &Lamps,
    corona: Corona,
    flash: &dyn Fn(u64) -> Option<FlashPattern>,
) -> Vec<Row> {
    rows(records, lamps, &to_car, &|_| Some(corona.clone()), flash, &mut |_| Some("CAR_CORONA0".to_owned()))
}

#[test]
fn the_state_bit_and_the_end_of_the_car_say_what_a_record_lights() {
    let lit = |bit, front| match role(&record(1, bit, None), front) {
        Some(Role::Lamp { lit, .. }) => lit,
        Some(Role::Siren) => "siren",
        None => "none",
    };
    assert_eq!((lit(0, true), lit(6, true), lit(4, true)), ("head", "head", "head"));
    assert_eq!((lit(1, false), lit(7, false), lit(0, false), lit(2, false)), ("brake", "tail", "tail", "reverse"));
    assert_eq!((lit(3, true), lit(3, false), lit(1, true), lit(2, true)), ("none", "none", "none", "none"));
    assert_eq!(
        role(&record(1, 3, Some(9)), false),
        Some(Role::Siren),
        "a bit 3 record with a flash pattern is a siren"
    );
}

#[test]
fn a_head_lamp_gives_a_flare_on_each_side_shining_forward_and_a_siren_gives_bars() {
    let lamps = lamps_with(&[(1, [0.7, 0.6, 2.2], FRONT), (2, [0.0, 1.4, -0.3], FRONT)]);
    let records = [record(1, 0, None), record(2, 3, Some(5))];
    let found = rows_of(&records, &lamps, corona(true, [0.2, 0.5, 1.0, 1.0]), &blink);
    let flares: Vec<_> = found.iter().filter(|row| matches!(row, Row::Flare { .. })).collect();
    assert_eq!(flares.len(), 2, "a locator and its mirror image");
    let sides: Vec<&str> =
        flares.iter().map(|row| if let Row::Flare { side, .. } = row { *side } else { "" }).collect();
    assert!(sides.contains(&"left") && sides.contains(&"right"));
    let Row::Flare { lit, look, .. } = flares[0] else { panic!() };
    assert_eq!(*lit, "head");
    assert!(look.direction[0] > 0.99, "a head lamp shines forward");
    assert!((look.falloff - 1.0).abs() < 0.3 || look.falloff > 0.0, "the angle lane becomes a cosine power");
    let bars: Vec<_> = found
        .iter()
        .filter_map(|row| {
            if let Row::Bar { siren, pattern, step, look } = row { Some((siren, pattern, step, look)) } else { None }
        })
        .collect();
    assert_eq!(bars.len(), 1, "a siren on the centre plane is one lamp: no mirror for mode 0");
    let (siren, pattern, step, look) = bars[0];
    assert_eq!(*siren, "blue");
    assert_eq!(pattern, "90909090", "a cycle of one time unit at 4 turns of two steps is eight steps");
    assert!((*step - 0.125).abs() < 1e-6);
    assert_eq!(look.falloff, 0.0, "a siren is seen from every side");
    assert_eq!(look.direction, [0.0, 0.0, 1.0]);
}

#[test]
fn a_lamp_invisible_on_its_axis_a_missing_picture_and_a_missing_pattern_leave_no_row() {
    let lamps = lamps_with(&[(1, [0.7, 0.6, 2.2], FRONT)]);
    let records = [record(1, 0, None)];
    assert!(rows_of(&records, &lamps, corona(false, [1.0; 4]), &no_flash).is_empty(), "a beam is not a flare");
    let mut no_picture = |_: u64| None;
    let found = rows(&records, &lamps, &to_car, &|_| Some(corona(true, [1.0; 4])), &no_flash, &mut no_picture);
    assert!(found.is_empty());
    let siren = [record(1, 3, Some(5))];
    assert!(rows_of(&siren, &lamps, corona(true, [1.0; 4]), &no_flash).is_empty(), "a siren needs its pattern");
    let nothing = rows(&records, &lamps, &to_car, &|_| None, &no_flash, &mut |_| Some("X".into()));
    assert!(nothing.is_empty(), "a record whose corona object is not in the bundle");
}

#[test]
fn the_brightness_is_relative_and_the_hue_is_the_layers() {
    let lamps = lamps_with(&[(1, [0.7, 0.6, -2.2], REAR)]);
    let records = [record(1, 1, None)];
    let found = rows_of(&records, &lamps, corona(true, [1.0, 0.1, 0.05, 1.0]), &no_flash);
    let Row::Flare { lit, look, .. } = &found[0] else { panic!() };
    assert_eq!(*lit, "brake");
    assert_eq!(look.colour[0], 1.0, "the brightest channel is 1");
    assert!(look.colour[1] < 0.2 && look.colour[2] < 0.1);
    assert_eq!(look.alpha[0].max(look.alpha[1]), 1.0, "the brighter of the two distances is 1");
    assert!(look.direction[0] < -0.99, "a brake lamp shines backward");
}

#[test]
fn a_siren_is_red_blue_or_white_by_its_tint() {
    assert_eq!(siren_colour([1.0, 0.1, 0.0]), "red");
    assert_eq!(siren_colour([0.2, 0.5, 1.0]), "blue");
    assert_eq!(siren_colour([1.0, 1.0, 1.0]), "white");
}

#[test]
fn the_manifest_text_is_valid_toml_and_the_game_reads_every_table() {
    let lamps = lamps_with(&[(1, [0.7, 0.6, 2.2], FRONT), (2, [0.0, 1.4, -0.3], FRONT)]);
    let records = [record(1, 0, None), record(2, 3, Some(5))];
    let found = rows_of(&records, &lamps, corona(true, [0.2, 0.5, 1.0, 1.0]), &blink);
    let text = manifest(&found);
    let parsed: toml::Table = toml::from_str(&text).unwrap();
    assert_eq!(parsed["lamp_flare"].as_array().map(Vec::len), Some(2));
    assert_eq!(parsed["light_bar"].as_array().map(Vec::len), Some(1));
    let car = blackbox_modding::content::parse_car("testcar", &text).unwrap();
    assert_eq!((car.lamp_flares.len(), car.light_bars.len()), (2, 1));
    assert_eq!(car.light_bars[0].pattern.len(), 8);
    assert!(manifest(&[]).is_empty());
}

fn segment(weight: f32, rate: f32, steps: &[f32]) -> Segment {
    Segment { weight, rate, offset: 0.0, steps: steps.to_vec() }
}

#[test]
fn a_pattern_is_baked_at_its_shortest_step_over_the_cycles_it_takes_to_repeat() {
    let one = FlashPattern { segments: vec![segment(1.0, 2.0, &[1.0, 0.0, 0.0, 0.0])] };
    let (digits, step) = pattern::bake(&one);
    assert!((step - 0.125).abs() < 1e-6);
    assert_eq!(digits, "90009000", "a cycle of one time unit is eight steps");
    // A segment that turns its sequence 3.2 times in its share of the cycle comes back after five cycles.
    let slow = FlashPattern { segments: vec![segment(8.0, 0.4, &[1.0, 0.0]), segment(8.0, 0.5, &[0.0, 1.0])] };
    let (digits, step) = pattern::bake(&slow);
    assert!((step - 1.0).abs() < 1e-6, "the steps are 1.25 and 1 s long: the shorter one");
    assert_eq!(digits.len(), 16 * 5, "5 cycles of 16 steps");
    assert!(digits.chars().all(|c| c.is_ascii_digit()));
    let (empty, step) = pattern::bake(&FlashPattern::default());
    assert_eq!((empty.len(), step), (1, 0.125));
}

#[test]
fn a_layers_list_is_the_size_the_reader_uses() {
    assert_eq!(LAYER_LEN, 0xE0);
}

/// The Charger cop's eight siren lamps and the Porsche's lamps come out of the install as rows with a pattern each
/// (`MW2012_GAME_DIR`).
#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_installs_police_cars_give_light_bars_and_every_car_gives_flares() {
    let Some(dir) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let root = std::path::PathBuf::from(dir);
    let install = |rel: &str| std::fs::read(root.join(rel)).ok();
    for (file, bars) in [("VEH_122706_HI.BNDL", 8), ("VEH_1085007_HI.BNDL", 0)] {
        let bytes = std::fs::read(root.join("VEHICLES").join(file)).unwrap();
        let lamps = chameleon_vehicle::read_lamps(&Bundle::parse(&bytes).unwrap()).unwrap();
        let built = build(&install, &bytes, &lamps, &to_car, "TESTCAR").expect("coronas");
        println!("{file}: {} flares, {} bars, {} sprites", built.flares, built.bars, built.sprites.len());
        assert!(built.flares > 0 && built.bars >= bars && !built.sprites.is_empty());
        let car = blackbox_modding::content::parse_car("testcar", &built.manifest).unwrap();
        assert_eq!(car.lamp_flares.len(), built.flares);
        assert_eq!(car.light_bars.len(), built.bars);
    }
}
