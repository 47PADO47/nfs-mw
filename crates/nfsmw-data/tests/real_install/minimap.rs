use nfsmw_data::minimap::{self, FULL_MAP, GRID};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

use crate::install;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_map_files_hold_an_eight_by_eight_grid_of_named_tiles() {
    let Some(dir) = install() else { return };
    for stem in [FULL_MAP, "MINI_MAP_Unlock_1", "MINI_MAP_Unlock_2"] {
        let tiles = minimap::read_tiles(&dir, stem).unwrap_or_else(|e| panic!("{stem}: {e:#}"));
        let header = stem.to_ascii_uppercase();
        for (n, t) in tiles.tiles.iter().enumerate() {
            assert!(t.width > 0 && t.width == t.height, "{stem} tile {n}: {}x{}", t.width, t.height);
            let name = blackbox_minimap::tile_name(&header, n as i32);
            assert_eq!(t.name_hash, blackbox_hash::bstring_hash(&name), "{stem} tile {n} is not {name}");
        }
        let sizes: std::collections::BTreeSet<_> = tiles.tiles.iter().map(|t| (t.width, t.height)).collect();
        eprintln!("{stem}: {} named tiles, texture sizes {sizes:?}", tiles.len());
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_open_city_calibration_covers_the_streamed_city() {
    let Some(dir) = install() else { return };
    let infos = minimap::read_track_infos(&dir).unwrap();
    assert_eq!(infos.len(), 18);
    let city = minimap::open_city_calibration(&dir).unwrap();
    assert!((city.origin[0] + 1224.8894).abs() < 1e-3 && (city.origin[1] + 1591.1449).abs() < 1e-3, "{city:?}");
    assert!((city.width - 6659.332).abs() < 1e-2, "{city:?}");

    // Every drivable section's centre lies on the picture.
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut checked = 0;
    for s in index.sections.iter().filter(|s| s.is_spatial() && s.number % 100 < 40) {
        let [u, v] = city.to_map(s.center);
        assert!((0.0..1.0).contains(&u) && (0.0..1.0).contains(&v), "{} at {:?} maps to ({u}, {v})", s.name, s.center);
        checked += 1;
    }
    assert!(checked > 80, "{checked}");
    assert_eq!(GRID.tiles_per_side, 8);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_event_map_file_holds_named_tiles_and_races_find_theirs() {
    let Some(dir) = install() else { return };
    let files: Vec<String> = dir
        .files_in("TRACKS/L2RA")
        .into_iter()
        .map(|f| f.rsplit_once('.').map_or(f.clone(), |(stem, _)| stem.to_owned()).to_ascii_uppercase())
        .filter(|stem| stem.starts_with("MINI_MAP_") && !stem.contains("UNLOCK"))
        .collect();
    assert_eq!(files.len(), 264, "the event map files");
    for stem in &files {
        let tiles = minimap::read_tiles(&dir, stem).unwrap_or_else(|e| panic!("{stem}: {e:#}"));
        for (n, t) in tiles.tiles.iter().enumerate() {
            let name = blackbox_minimap::tile_name(stem, n as i32);
            assert_eq!(t.name_hash, blackbox_hash::bstring_hash(&name), "{stem} tile {n} is not {name}");
        }
    }
    // A race that is not a pursuit race names its file by its event id; the few without a file keep the stage's map.
    let data = nfsmw_data::gameplay::load(&dir).unwrap();
    let (mut with, mut without) = (0, Vec::new());
    for race in data.races.iter().filter(|r| !r.params.pursuit_race) {
        let event = race.event.clone().unwrap_or_default();
        let stem = minimap::event_stem(&event);
        match files.contains(&stem) {
            true => with += 1,
            false => without.push(event),
        }
    }
    eprintln!("{with} races with an event map; without: {without:?}");
    assert!(with > 200 && without.len() < 15, "{with} with, {} without", without.len());
}
