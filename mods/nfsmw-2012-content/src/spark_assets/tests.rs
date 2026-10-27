use chameleon_effects::collision::Bounce;

use super::profile::{num, sparks, xenon};
use super::*;

fn streak() -> Spark {
    Spark {
        kind: Kind::Streak,
        life: [0.25, 0.85],
        rgb: [[1.0, 0.41, 0.15]; 4],
        brightness: [12.0, 6.0, 6.0, 4.8],
        alpha: [1.0, 1.0, 1.0, 0.5],
        width: [0.08, 0.04, 0.02, 0.01],
        speed: [4.0, 8.0],
        tilt: 10.0,
        inherit: [0.0, 1.1],
        density: 200.0,
        patch_radius: 0.5,
        bounce: Some(Bounce {
            restitution: [0.8, 0.9],
            friction: [0.01, 0.05],
            gravity: -9.8,
            drag: 0.4,
            wind_blend: 0.0,
            turbulence: 0.0,
        }),
        stretch: Some(0.125),
        sheet: None,
    }
}

fn set() -> Set {
    let hit = Channel { gate: Gate::Impact { floor: 0.153, full_speed: 22.08 }, sparks: vec![streak()] };
    let scrape = Channel { gate: Gate::Scrape { start: 0.73, full: 1.37 }, sparks: vec![streak()] };
    Set { hit, scrape }
}

#[test]
fn numbers_are_written_short() {
    assert_eq!(num(0.125), "0.125");
    assert_eq!(num(8.0), "8.0");
    assert_eq!(num(-0.00001), "0.0");
    assert_eq!(num(22.080002), "22.08");
}

#[test]
fn both_profiles_are_valid_toml_with_the_gates_and_emitters() {
    for text in [sparks(&set()), xenon(&set())] {
        let table: toml::Table = text.parse().expect("the profile parses");
        assert_eq!(table["format"].as_integer(), Some(1));
        let hit = table["hit"].as_table().unwrap();
        assert!((hit["closing_speed_full"].as_float().unwrap() - 22.08).abs() < 1e-3);
        assert!((hit["intensity_floor"].as_float().unwrap() - 0.153).abs() < 1e-3);
        let scrape = table["scrape"].as_table().unwrap();
        assert!((scrape["speed_start"].as_float().unwrap() - 0.73).abs() < 1e-3);
        assert!((scrape["speed_full"].as_float().unwrap() - 1.37).abs() < 1e-3);
    }
}

#[test]
fn the_original_style_profile_has_the_host_units() {
    let text = sparks(&set());
    let table: toml::Table = text.parse().unwrap();
    let emitter = &table["hit"]["layer"][0];
    // 200 a square metre over half a disc of 0.5 m, rounded.
    assert_eq!(emitter["burst"].as_float(), Some(79.0));
    assert!((emitter["life_variance"].as_float().unwrap() - (1.0 - 0.25 / 0.85)).abs() < 1e-3);
    assert_eq!(emitter["gravity"].as_float(), Some(9.8));
    assert_eq!(emitter["colors"][0].as_array().unwrap().len(), 4);
    assert_eq!(emitter["stretch"].as_float(), Some(0.125));
    // A loss of 40 percent a second at 6 m/s.
    let drag = emitter["drag"].as_float().unwrap();
    assert!((drag - 0.0851).abs() < 1e-3, "{drag}");
    let rate = &table["scrape"]["layer"][0];
    assert!(rate.get("rate").is_some() && rate.get("burst").is_none());
}

#[test]
fn the_restored_style_profile_has_the_line_spark_fields() {
    let text = xenon(&set());
    let table: toml::Table = text.parse().unwrap();
    let spark = &table["hit"]["spark"][0];
    assert_eq!(spark["length"].as_float(), Some(255.0));
    assert_eq!(spark["height"].as_float(), Some(77.0));
    assert_eq!(spark["gravity"].as_float(), Some(-4.9));
    assert_eq!(spark["elasticity"].as_float(), Some(217.0));
    assert_eq!(spark["velocity"][2].as_float(), Some(6.0));
    assert!(table.get("trail").is_none());
}

#[test]
fn the_games_parsers_accept_both_profiles() {
    use blackbox_modding::content::{parse_sparks_profile, parse_xenon_profile};
    let sparks = parse_sparks_profile(&sparks(&set())).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((sparks.hit.items.len(), sparks.scrape.items.len()), (1, 1));
    let layer = &sparks.hit.items[0];
    assert_eq!((layer.count, layer.stretch, layer.keys.len()), (79.0, 0.125, 4));
    assert_eq!(layer.brightness, vec![12.0, 6.0, 6.0, 4.8]);
    assert!((sparks.hit.strength(22.08).unwrap() - 1.0).abs() < 1e-3);
    assert!((sparks.scrape.strength(1.37).unwrap() - 1.0).abs() < 1e-3);
    let xenon = parse_xenon_profile(&xenon(&set())).unwrap_or_else(|e| panic!("{e}"));
    let spark = &xenon.hit.items[0];
    assert_eq!((spark.length, spark.height, spark.gravity, spark.elasticity), (255.0, 77.0, -4.9, Some(217.0)));
    assert!((xenon.hit.strength(0.0).unwrap() - 0.153).abs() < 1e-3);
}

#[test]
fn a_sprite_names_a_picture_the_mod_writes_and_the_games_parser_takes_it() {
    use blackbox_modding::content::parse_sparks_profile;
    let flash = Spark {
        kind: Kind::Sprite,
        stretch: None,
        sheet: Some(Sheet {
            name: "E1_0.PNG".to_owned(),
            picture: crate::vfx::Picture { width: 1, height: 1, rgba: vec![255; 4] },
            grid: 4,
        }),
        ..streak()
    };
    let mut set = set();
    set.hit.sparks.push(flash);
    let parsed = parse_sparks_profile(&sparks(&set)).unwrap_or_else(|e| panic!("{e}"));
    let sprite = &parsed.hit.items[1];
    assert_eq!((sprite.grid, sprite.texture.as_deref()), (4, Some("SPARKS/MW2012/E1_0.PNG")));
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_real_install_gives_a_hit_and_a_scrape_channel() {
    let root = std::path::PathBuf::from(std::env::var_os("MW2012_GAME_DIR").expect("MW2012_GAME_DIR"));
    let data = std::fs::read(root.join(GLOBAL_EFFECTS)).unwrap();
    let bundle = Bundle::parse(&data).unwrap();
    let set = read(&bundle).unwrap();
    let Gate::Impact { floor, full_speed } = set.hit.gate else { panic!("the hit gate is an impact") };
    assert!((floor - 0.153).abs() < 0.01 && (full_speed - 22.08).abs() < 0.1);
    let Gate::Scrape { start, full } = set.scrape.gate else { panic!("the scrape gate is a scrape") };
    assert!((start - 0.73).abs() < 0.02 && (full - 1.37).abs() < 0.02);
    assert!(set.hit.sparks.iter().any(|s| s.kind == Kind::Streak && s.stretch == Some(0.125)));
    assert!(set.hit.sparks.iter().any(|s| s.sheet.is_some()), "the flash has a picture");
    assert!(!set.scrape.sparks.is_empty());
    for text in [sparks(&set), xenon(&set)] {
        assert!(text.parse::<toml::Table>().is_ok());
    }
}
