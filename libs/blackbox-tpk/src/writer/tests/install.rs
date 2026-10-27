//! Re-writing a real car `TEXTURES.BIN`. Skipped when there is no install; never commits game data.

use std::path::PathBuf;

use crate::{PackForm, TextureSource, WriteOptions, read_texture_packs, write_tpk_with};

const DEFAULT_INSTALL: &str = r"D:\Need For Speed Most Wanted Black Edition";

/// The first `CARS/*/TEXTURES.BIN` of the install, if there is one.
fn car_textures() -> Option<PathBuf> {
    let root = std::env::var_os("NFSMW_GAME_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_INSTALL));
    let mut cars: Vec<PathBuf> =
        std::fs::read_dir(root.join("CARS")).ok()?.filter_map(|e| Some(e.ok()?.path())).collect();
    cars.sort();
    cars.into_iter().map(|c| c.join("TEXTURES.BIN")).find(|p| p.is_file())
}

#[test]
fn a_real_car_pack_round_trips_in_both_forms() {
    let Some(path) = car_textures() else {
        eprintln!("skipped: no game install (set NFSMW_GAME_DIR)");
        return;
    };
    let original = &read_texture_packs(&std::fs::read(path).unwrap()).unwrap()[0];
    assert!(!original.textures.is_empty());
    assert!(original.failed.is_empty());
    let sources: Vec<TextureSource> = original.textures.iter().map(TextureSource::from).collect();
    let mut expected = original.textures.clone();
    expected.sort_by_key(|t| t.name_hash);

    for form in [PackForm::Streamed, PackForm::Plain] {
        let options = WriteOptions { form, filename: Some(original.filename.clone()) };
        let file = write_tpk_with(&original.name, &sources, &options).unwrap();
        let again = &read_texture_packs(&file).unwrap()[0];
        assert!(again.failed.is_empty());
        assert_eq!((&again.name, &again.filename), (&original.name, &original.filename));
        assert_eq!(again.textures, expected);

        // Writing what was just read is stable.
        let sources_again: Vec<TextureSource> = again.textures.iter().map(TextureSource::from).collect();
        assert_eq!(write_tpk_with(&original.name, &sources_again, &options).unwrap(), file);
    }
}
