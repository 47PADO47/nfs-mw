//! Every bundle of a real install parses, inflates and has import tables inside its blocks.
//! Set `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`.

use std::path::{Path, PathBuf};

use crate::{Bundle, FLAG_HAS_DEBUG_DATA, FLAG_HAS_DEFAULT_RESOURCE, FLAG_MULTISTREAM, IdClass};

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

/// The header and table facts of `docs/formats/chameleon-bundle.md`, checked on every bundle (nothing is inflated).
#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_bundle_header_and_table_follows_the_spec() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut files = Vec::new();
    bundles(Path::new(&dir), &mut files);
    assert!(files.len() > 2000, "expected the 2,021 bundles of the 2012 install, found {}", files.len());
    for path in files {
        let data = std::fs::read(&path).unwrap();
        let bundle = Bundle::parse(&data).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let header = bundle.header();
        let named = (0..4).filter(|&stream| header.stream_name(stream).is_some()).count();
        // The default resource is set exactly when its flag is.
        assert_eq!(header.has_flags(FLAG_HAS_DEFAULT_RESOURCE), header.default_resource != 0, "{}", path.display());
        // Only multi-stream bundles name streams, and every entry points at a named one.
        assert_eq!(header.has_flags(FLAG_MULTISTREAM), named > 0, "{}", path.display());
        for entry in bundle.entries() {
            assert_eq!(entry.flags, 0, "{}", path.display());
            let in_range = usize::from(entry.stream_index) < named.max(1);
            assert!(in_range, "{}: stream {} of {named}", path.display(), entry.stream_index);
            let class = entry.resource_id().class();
            assert!(matches!(class, IdClass::Normal | IdClass::GameChanger), "{}: {class:?}", path.display());
        }
        // Debug data parses to at least one name.
        if header.has_flags(FLAG_HAS_DEBUG_DATA) {
            assert!(!bundle.debug_names().is_empty(), "{}", path.display());
        }
    }
}
