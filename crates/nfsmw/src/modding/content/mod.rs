//! Content mods (ADR 0005 section 5): the cars, the music, the racer names, the map packs and the interiors a mod declares in files. The
//! mod's `files/` folder is already a layer of the install (`layers.rs`). This module reads the mod's `cars/` and `music/`
//! declarations for the loaded mods, and seals the results once, at start-up, before any scene is built. The map packs
//! and the interiors are read when a world opens, against its streaming index (`scenery.rs`, `interior/`).
//!
//! The rules and the file layout are in docs/modding/api-content.md.

mod cars;
pub mod interior;
mod music;
mod racers;
pub mod scenery;

pub(crate) use music::{MusicSource, sources as music_sources};
pub(crate) use racers::names as racer_names;

use std::path::PathBuf;
use std::sync::OnceLock;

use blackbox_modding::host::Candidate;
use game_install::GameDir;

/// The loaded mods of this run, sealed by [`register`]. The world reads them when it opens.
static MODS: OnceLock<Vec<ModDir>> = OnceLock::new();

/// A loaded mod and the folder it was found in: where its `cars/` and its `mod.toml` (with `[music]`) are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDir {
    pub id: String,
    pub dir: PathBuf,
}

/// The loaded mods that have a folder, in load order. A mod that is not loaded declares nothing.
pub fn plan(loaded: &[String], candidates: &[Candidate]) -> Vec<ModDir> {
    loaded
        .iter()
        .filter_map(|id| {
            let candidate = candidates.iter().find(|candidate| &candidate.manifest.id == id)?;
            Some(ModDir { id: id.clone(), dir: candidate.dir.clone() })
        })
        .collect()
}

/// Read the declarations of `mods` and seal them: the mod cars into the car tables, the music sources into the custom
/// music library. `dir` is the install with the mods' layers in place, which says which car files a mod supplies.
pub fn register(dir: &GameDir, mods: &[ModDir]) {
    cars::register(dir, mods);
    music::register(mods);
    racers::register(mods);
    if MODS.set(mods.to_vec()).is_err() {
        log::warn!("mod content was already registered for this run; the first list is kept");
    }
}

/// The loaded mods of this run, in load order. Empty before [`register`].
pub fn mod_dirs() -> &'static [ModDir] {
    MODS.get().map(Vec::as_slice).unwrap_or(&[])
}

/// The `.toml` files directly in `folder`, sorted by name. A missing folder has none.
fn toml_files(folder: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut files: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("toml")))
        .collect();
    files.sort();
    files
}
