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
