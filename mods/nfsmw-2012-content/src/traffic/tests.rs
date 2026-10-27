use std::path::PathBuf;

use blackbox_modding::content::{TrafficClass, parse_car};

use super::*;
use crate::import::Report;

/// A mod folder in the temporary folder for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-traffic-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_traffic_setting_is_on_by_default_and_turns_the_cars_off() {
    assert!(Config::default().traffic);
    assert!(!Config::parse("traffic = false\n").unwrap().traffic);
}

#[test]
fn the_selection_of_cars_picks_traffic_cars_by_id_or_by_the_word_traffic() {
    let id = "AUDA3_TR";
    let display = "Audi A3 (traffic)";
    assert!(Config::default().wants(id, display));
    assert!(Config::parse("cars = ['traffic']\n").unwrap().wants(id, display));
    assert!(Config::parse("cars = ['AUDA3_TR']\n").unwrap().wants(id, display));
    assert!(!Config::parse("cars = ['Porsche']\n").unwrap().wants(id, display), "a list of other cars: no traffic");
    assert!(!Config::parse("exclude = ['Audi']\n").unwrap().wants(id, display));
}

/// The eleven traffic bundles of the install convert to eleven traffic cars the game's parser reads, with the files the game
/// needs. Set `MW2012_GAME_DIR` to the 2012 install and run with `--ignored`; it prints the time and the size.
#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_eleven_traffic_bundles_convert_to_traffic_cars() {
    let Some(install) = std::env::var_os("MW2012_GAME_DIR") else { return };
    let source = Source::open(std::path::Path::new(&install)).unwrap();
    assert_eq!(bundles(&source).len(), 11);
    let out = scratch("real");
    let started = std::time::Instant::now();
    let mut report = Report::default();
    import_all(&source, &Config::default(), &out, &mut report);
    assert!(report.skipped.is_empty(), "left out: {:?}", report.skipped);
    assert_eq!(report.traffic.len(), 11);
    let mut bytes = 0;
    let mut classes = Vec::new();
    for car in &report.traffic {
        let text =
            std::fs::read_to_string(out.join("cars").join(format!("{}.toml", car.id.to_ascii_lowercase()))).unwrap();
        let manifest = parse_car(&car.id, &text).unwrap_or_else(|error| panic!("{}: {error}\n{text}", car.id));
        assert!(manifest.role.is_traffic() && !manifest.role.is_player(), "{}", car.id);
        classes.push(manifest.traffic.as_ref().unwrap().class);
        assert!(manifest.wheels.is_some() && manifest.customization.is_some(), "{}", car.id);
        let folder = out.join("files/CARS").join(&car.id);
        let geometry = std::fs::read(folder.join("GEOMETRY.BIN")).unwrap();
        let (solids, errors) = blackbox_solid::read_solids_lossy(&geometry).unwrap();
        assert!(errors.is_empty(), "{}: {errors:?}", car.id);
        for stem in ["KIT00_BODY", "KIT00_FRONT_TIRE"] {
            for letter in ['A', 'B', 'C', 'D'] {
                let name = format!("{}_{stem}_{letter}", car.id);
                assert!(solids.iter().any(|solid| solid.name == name), "no solid named {name}");
            }
        }
        let textures = std::fs::read(folder.join("TEXTURES.BIN")).unwrap();
        assert!(blackbox_tpk::read_texture_packs(&textures).unwrap().iter().all(|pack| pack.failed.is_empty()));
        assert!(folder.join("BOUNDS.BIN").is_file(), "{}: a car without bounds cannot drive", car.id);
        bytes += std::fs::metadata(folder.join("GEOMETRY.BIN")).unwrap().len()
            + std::fs::metadata(folder.join("TEXTURES.BIN")).unwrap().len()
            + std::fs::metadata(folder.join("BOUNDS.BIN")).unwrap().len();
    }
    for (class, count) in [
        (TrafficClass::Compact, 4),
        (TrafficClass::Sedan, 2),
        (TrafficClass::Van, 2),
        (TrafficClass::Pickup, 2),
        (TrafficClass::Truck, 1),
    ] {
        assert_eq!(classes.iter().filter(|found| **found == class).count(), count, "{class:?}");
    }
    println!(
        "{} traffic cars in {:.2} s, {:.1} MB",
        report.traffic.len(),
        started.elapsed().as_secs_f32(),
        bytes as f32 / (1024.0 * 1024.0)
    );
    std::fs::remove_dir_all(out).unwrap();
}
