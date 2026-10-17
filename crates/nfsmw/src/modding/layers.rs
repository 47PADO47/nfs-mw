//! The `files/` folders of the loaded mods, as layers of the install (ADR 0005 section 5).

use std::path::PathBuf;

use blackbox_modding::host::Candidate;
use game_install::GameDir;

/// The folder inside a mod that mirrors the install layout.
pub const FILES_FOLDER: &str = "files";

/// One mod's layer: the folder that replaces or adds to the install's files, and its priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub id: String,
    pub root: PathBuf,
    pub priority: i32,
}

/// The layers of the loaded mods, lowest first. `loaded` is the load order; `candidates` are the discovered mods,
/// which give each one's folder and priority. A mod without a `files/` folder adds no layer. Ties keep the load order,
/// so the mod loaded later wins.
pub fn plan(loaded: &[String], candidates: &[Candidate]) -> Vec<Layer> {
    let mut layers: Vec<Layer> = loaded
        .iter()
        .filter_map(|id| {
            let candidate = candidates.iter().find(|candidate| &candidate.manifest.id == id)?;
            let root = candidate.dir.join(FILES_FOLDER);
            if !root.is_dir() {
                return None;
            }
            Some(Layer { id: id.clone(), root, priority: candidate.manifest.priority })
        })
        .collect();
    layers.sort_by_key(|layer| layer.priority);
    layers
}

/// Add the layers to the install, in the order given. A layer that cannot be indexed is logged and skipped: a broken
/// mod folder does not stop the game.
pub fn apply(dir: &mut GameDir, layers: &[Layer]) {
    for layer in layers {
        if let Err(error) = dir.add_overlay(&layer.id, &layer.root, layer.priority) {
            log::warn!("mod {}: its {FILES_FOLDER} folder is not added: {error}", layer.id);
        }
    }
}
