//! Imports one car from real installs and reads the result back with the game's own readers.
//! Set `MW2012_GAME_DIR` (the 2012 install) and `NFSMW_GAME_DIR` (the 2005 install) and run with `--ignored`.

use std::path::PathBuf;

use nfsmw_import2012::config::{Config, Selection};

#[test]
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR)"]
fn an_imported_car_reads_back_with_the_games_readers() {
    let (Some(source), Some(game)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let out = std::env::temp_dir().join(format!("nfsmw-import2012-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let config = Config {
        source: Some(PathBuf::from(source)),
        game_dir: Some(PathBuf::from(game)),
        output: Some(out.clone()),
        cars: Selection::Only(vec!["POR911CARR12".into()]),
        ..Config::default()
    };
    let report = nfsmw_import2012::run(&config).unwrap();
    assert_eq!(report.imported.len(), 1, "skipped: {:?}", report.skipped);
    let car = &report.imported[0];

    let folder = out.join("files/CARS").join(&car.id);
    let geometry = std::fs::read(folder.join("GEOMETRY.BIN")).unwrap();
    let (solids, errors) = blackbox_solid::read_solids_lossy(&geometry).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    for stem in ["KIT00_BODY", "KIT00_FRONT_TIRE"] {
        for letter in ['A', 'B', 'C', 'D'] {
            let suffix = format!("_{stem}_{letter}");
            assert!(solids.iter().any(|solid| solid.name.ends_with(&suffix)), "no solid ending {suffix}");
        }
    }
    assert!(solids.iter().all(|solid| !solid.groups.is_empty()));

    let textures = std::fs::read(folder.join("TEXTURES.BIN")).unwrap();
    let packs = blackbox_tpk::read_texture_packs(&textures).unwrap();
    assert!(packs.iter().all(|pack| pack.failed.is_empty()));
    assert!(packs.iter().map(|pack| pack.textures.len()).sum::<usize>() > 0);

    let manifest =
        std::fs::read_to_string(out.join("cars").join(format!("{}.toml", car.id.to_ascii_lowercase()))).unwrap();
    assert!(manifest.contains(&format!("clone = \"{}\"", car.clone)));
    assert!(out.join("mod.toml").is_file());
    std::fs::remove_dir_all(&out).unwrap();
}
