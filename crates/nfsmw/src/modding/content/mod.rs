//! Content mods (ADR 0005 section 5): the cars and the music a mod declares in files. The mod's `files/` folder is
//! already a layer of the install (`layers.rs`). This module reads the mod's `cars/` and `music/` declarations for the
//! loaded mods, and seals the results once, at start-up, before any scene is built.
//!
//! The rules and the file layout are in docs/modding/api-content.md.

mod cars;
mod music;

pub(crate) use music::{MusicSource, sources as music_sources};

use std::path::PathBuf;

use blackbox_modding::host::Candidate;
use game_install::GameDir;

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
}
