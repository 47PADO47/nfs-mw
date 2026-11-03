//! Every manifest of an imported mod folder reads back with the game's content schema, and every car's files are there.
//!
//! The folder is the user's own output of `import_all.py` (`docs/modding/ride6-bikes-batch.md`), so the test is ignored unless
//! `RIDE6_MOD_DIR` names it (a path relative to mods/ride6-bikes, where the tests run, or absolute): `RIDE6_MOD_DIR=. cargo test -p ride6-bikes --test imported_manifests -- --ignored`.

use std::path::{Path, PathBuf};

use blackbox_modding::content::{CarKind, parse_car};

fn mod_dir() -> Option<PathBuf> {
    std::env::var_os("RIDE6_MOD_DIR").map(PathBuf::from)
}

fn manifests(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir.join("cars"))
        .expect("the mod folder has a cars/ folder")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    found.sort();
    found
}

#[test]
#[ignore = "needs the imported mod folder (set RIDE6_MOD_DIR)"]
fn every_imported_manifest_is_a_valid_bike_with_its_files() {
    let Some(dir) = mod_dir() else { return };
    let mut failures = Vec::new();
    let mut ids = std::collections::HashSet::new();
    let list = manifests(&dir);
    for path in &list {
        let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
        let text = std::fs::read_to_string(path).expect("a readable manifest");
        match parse_car(stem, &text) {
            Err(error) => failures.push(format!("{stem}: {error}")),
            Ok(manifest) if manifest.kind != CarKind::Bike => failures.push(format!("{stem}: not a bike")),
            Ok(_) => {}
        }
        let folder = dir.join("files").join("CARS").join(stem.to_ascii_uppercase());
        for file in ["GEOMETRY.BIN", "TEXTURES.BIN", "BOUNDS.BIN"] {
            if !folder.join(file).is_file() {
                failures.push(format!("{stem}: {file} is missing"));
            }
        }
        if !ids.insert(stem.to_ascii_uppercase()) || stem.len() > 27 {
            failures.push(format!("{stem}: id repeated or longer than 27"));
        }
    }
    assert!(failures.is_empty(), "{} of {} manifests failed:\n{}", failures.len(), list.len(), failures.join("\n"));
}
