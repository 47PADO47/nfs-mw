//! The ignition sequences of the 2012 install's cars. Set `CHAMELEON_GAME_DIR` (or `MW2012_GAME_DIR`) to the install folder
//! and run with `--ignored --nocapture` to print the table the doc quotes.

use std::path::{Path, PathBuf};

use chameleon_bundle::Bundle;

use crate::read_ignition;

/// `VEHICLES/VEH_<n>_HI.BNDL` of the install, sorted.
fn car_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("VEHICLES"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy().to_uppercase();
            name.starts_with("VEH_") && name.ends_with("_HI.BNDL")
        })
        .collect();
    files.sort();
    files
}

fn install_dir() -> Option<PathBuf> {
    std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR")).map(PathBuf::from)
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn a_car_with_an_ignition_sequence_plays_one_short_sample() {
    let Some(dir) = install_dir() else { return };
    let files = car_files(&dir);
    assert_eq!(files.len(), 93, "the 2012 install has 93 cars");
    let shared = std::fs::read(dir.join("VEHICLES").join("WAVES.BNDL")).unwrap();
    let shared = Bundle::parse(&shared).unwrap();
    let mut with = 0;
    let mut distinct = std::collections::BTreeSet::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let data = std::fs::read(&path).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let Some(ignition) = read_ignition(&bundle) else {
            println!("{name}: no ignition sequence");
            continue;
        };
        with += 1;
        println!(
            "{name}: sequence {:x} sample {:x} {:.3} s gain {} effect {:?}",
            ignition.sequence & 0xff_ffff,
            ignition.sample & 0xff_ffff,
            ignition.seconds,
            ignition.gain,
            ignition.effect_window
        );
        // The sample is in the car's bundle or in the samples the cars share (`VEHICLES/WAVES.BNDL`).
        let found = bundle.find(ignition.sample).is_some() || shared.find(ignition.sample).is_some();
        assert!(found, "{name}: no bundle holds the sample");
        distinct.insert(ignition.sample);
        assert!((0.1..=3.0).contains(&ignition.seconds), "{name}: {} s", ignition.seconds);
        assert!(ignition.gain > 0.0 && ignition.gain <= 16.0, "{name}: gain {}", ignition.gain);
    }
    println!("{with} cars have an ignition sequence, {} distinct samples", distinct.len());
    assert!(with >= 20, "{with} cars have an ignition sequence");
}
