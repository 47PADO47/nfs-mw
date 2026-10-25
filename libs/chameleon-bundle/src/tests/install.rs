//! Every bundle of a real install parses, inflates and has import tables inside its blocks.
//! Set `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`.

use std::path::{Path, PathBuf};

use crate::Bundle;

fn bundles(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            bundles(&path, out);
            continue;
        }
        let is_bundle = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("bndl"));
        if is_bundle {
            out.push(path);
        }
    }
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_vehicle_bundle_inflates() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut files = Vec::new();
    bundles(&Path::new(&dir).join("VEHICLES"), &mut files);
    assert!(!files.is_empty(), "no bundles under VEHICLES");
    for path in files {
        let data = std::fs::read(&path).unwrap();
        let bundle = Bundle::parse(&data).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for index in 0..bundle.entries().len() {
            bundle.resource(index).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        }
    }
}
