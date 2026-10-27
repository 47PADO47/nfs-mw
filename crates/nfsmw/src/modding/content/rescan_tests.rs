//! A mod that produces content after start-up: its `files/` layer reaches the copies of the install that were made
//! before, and its cars join the car list. Needs the install (`NFSMW_GAME_DIR`), like the other tests of the car tables.

use std::path::{Path, PathBuf};

use nfsmw_data::car::{CarTables, mods as car_mods};

use super::{ModDir, RescanError, rescan_in};

fn mod_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nfsmw-rescan-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("files/CARS/LATECAR")).unwrap();
    std::fs::create_dir_all(dir.join("cars")).unwrap();
    std::fs::write(dir.join("files/CARS/LATECAR/GEOMETRY.BIN"), b"not read by the registration").unwrap();
    std::fs::write(dir.join("cars/latecar.toml"), "name = \"Late car\"\n").unwrap();
    dir
}

fn install() -> Option<game_install::GameDir> {
    let root = std::env::var_os("NFSMW_GAME_DIR")?;
    game_install::GameDir::open(Path::new(&root)).ok()
}

#[test]
fn content_made_after_start_up_reaches_the_scenes_that_are_already_open() {
    let Some(dir) = install() else { return };
    let folder = mod_folder("late");
    let mods = [ModDir { id: "example.late".to_owned(), dir: folder.clone(), priority: 0 }];
    let held_by_a_scene = dir.clone();
    assert!(!held_by_a_scene.exists("CARS/LATECAR/GEOMETRY.BIN"));
    assert!(!car_mods::published().iter().any(|car| car.folder == "LATECAR"));

    assert_eq!(rescan_in(&dir, &mods, "example.late"), Ok(1));
    assert!(held_by_a_scene.exists("CARS/LATECAR/GEOMETRY.BIN"), "the copy made earlier sees the new layer");
    assert!(car_mods::published().iter().any(|car| car.folder == "LATECAR"));
    let tables = CarTables::load(&held_by_a_scene).unwrap();
    assert!(tables.car_type_for_folder("LATECAR").is_some(), "a car table built afterwards lists the car");

    assert_eq!(rescan_in(&dir, &mods, "example.late"), Ok(0), "a second rescan adds nothing");
    assert_eq!(rescan_in(&dir, &mods, "example.other"), Err(RescanError::NotLoaded));
    std::fs::remove_dir_all(folder).unwrap();
}
