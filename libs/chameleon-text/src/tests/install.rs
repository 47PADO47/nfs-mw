//! The 14 language bundles of a real install read, and write back to the same bytes.
//! Set `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`.

use std::path::Path;

use chameleon_bundle::{Bundle, ResourceKind};

use crate::StringTable;

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_language_round_trips_byte_for_byte() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let mut counts = Vec::new();
    for language in 1..=14 {
        let path = Path::new(&dir).join("UI/LANGUAGE").join(format!("{language:04}.BNDL"));
        let data = std::fs::read(&path).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        assert_eq!(bundle.entries().len(), 1, "{}", path.display());
        let resource = bundle.resource(0).unwrap();
        assert_eq!(resource.entry.kind, ResourceKind::LocalisedText.id());
        let block = &resource.blocks[0];
        let table = StringTable::parse(block).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(table.language(), language);
        assert_eq!(&table.to_bytes(), block, "{} does not write back to its own bytes", path.display());
        counts.push(table.entries().len());
    }
    // Every language has the same string ids.
    assert!(counts.windows(2).all(|pair| pair[0] == pair[1]), "{counts:?}");
}
