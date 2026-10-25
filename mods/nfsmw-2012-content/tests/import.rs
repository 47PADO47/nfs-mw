//! Converts one car from real installs, the way the mod does at start-up, and reads the result back with the game's own
//! readers. Set `MW2012_GAME_DIR` (the 2012 install) and `NFSMW_GAME_DIR` (the 2005 install) and run with `--ignored`.

use std::path::{Path, PathBuf};

use nfsmw_import2012::{Outcome, prepare};

fn mod_folder(source: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-import2012-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let source = source.to_string_lossy().replace('\\', "/");
    std::fs::write(dir.join("config.toml"), format!("source = '{source}'\ncars = ['POR911CARR12']\n")).unwrap();
    dir
}

#[test]
#[ignore = "reads both game installs (MW2012_GAME_DIR, NFSMW_GAME_DIR)"]
fn a_start_converts_the_cars_and_the_next_start_does_not() {
    let (Some(source), Some(game)) = (std::env::var_os("MW2012_GAME_DIR"), std::env::var_os("NFSMW_GAME_DIR")) else {
        return;
    };
    let (source, game) = (PathBuf::from(source), PathBuf::from(game));
    let dir = mod_folder(&source);

    let Outcome::Imported(report) = prepare(&dir, &game).unwrap() else { panic!("the first start must convert") };
    assert_eq!(report.imported.len(), 1, "skipped: {:?}", report.skipped);
    let car = &report.imported[0];

    let folder = dir.join("files/CARS").join(&car.id);
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
        std::fs::read_to_string(dir.join("cars").join(format!("{}.toml", car.id.to_ascii_lowercase()))).unwrap();
    assert!(manifest.contains(&format!("clone = \"{}\"", car.clone)));

    assert!(matches!(prepare(&dir, &game).unwrap(), Outcome::UpToDate { cars: 1 }));
    std::fs::write(dir.join("config.toml"), "source = 'nowhere'\n").unwrap();
    assert!(prepare(&dir, &game).is_err(), "a changed setting must be read, and a missing install is an error");
    std::fs::remove_dir_all(&dir).unwrap();
}
