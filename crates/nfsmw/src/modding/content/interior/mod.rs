//! The interiors of the mods (docs/specs/mod-world-content.md, section 3). Each `interiors/<id>.toml` names a scenery pack of
//! the same mod, which is the place, and an entry zone and an exit zone. The world reads them when it opens, against the
//! packs it has (`load`), and the steps of an entry are in `machine`.

pub mod machine;
#[cfg(test)]
mod tests;

use std::path::Path;

use blackbox_modding::content::{Exit, INTERIORS_FOLDER, Pose, Shape, Suspend, TimeOverride, parse_interior};

use super::scenery::MapPack;
use super::{ModDir, toml_files};

/// A checked interior whose pack is loaded: what the world needs to offer it and to carry it out.
#[derive(Debug, Clone, PartialEq)]
pub struct ModInterior {
    /// The mod that declares the interior.
    pub owner: String,
    /// The interior key: the manifest's file name.
    pub key: String,
    /// The name the entry card shows.
    pub label: String,
    /// The entry card's line, if the manifest gives one.
    pub prompt: Option<String>,
    /// The entry zone, in render space.
    pub entry: Shape,
    /// The residency index of the interior's pack.
    pub tile: usize,
    pub spawn: Pose,
    pub exit: Exit,
    pub time: Option<TimeOverride>,
    pub suspend: Suspend,
}

/// The interiors of `dirs`, in load order and then file-name order. `packs` are the map packs of this run, and `first_tile`
/// is the residency index of the first of them. An interior whose pack is not loaded is left out with the reason.
pub fn load(dirs: &[ModDir], packs: &[MapPack], first_tile: usize) -> Vec<ModInterior> {
    dirs.iter().flat_map(|m| interiors_of(m, packs, first_tile)).collect()
}

fn interiors_of(m: &ModDir, packs: &[MapPack], first_tile: usize) -> Vec<ModInterior> {
    let mut out = Vec::new();
    for path in toml_files(&m.dir.join(INTERIORS_FOLDER)) {
        match read(m, &path, packs, first_tile) {
            Ok(interior) => {
                log::info!(
                    "mod {}: interior {} uses the pack at residency index {}",
                    m.id,
                    interior.key,
                    interior.tile
                );
                out.push(interior);
            }
            Err(reason) => log::warn!("mod {}: interior {} is not added: {reason}", m.id, path.display()),
        }
    }
    out
}

/// Read and check one interior. The error is the reason, for the log.
fn read(m: &ModDir, path: &Path, packs: &[MapPack], first_tile: usize) -> Result<ModInterior, String> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or("the file name is not text")?;
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let manifest = parse_interior(stem, &text).map_err(|error| error.to_string())?;
    let at = packs.iter().position(|pack| pack.owner == m.id && pack.key == manifest.pack).ok_or_else(|| {
        format!("its pack {} is not loaded: the pack must be a scenery manifest of the same mod", manifest.pack)
    })?;
    if manifest.time.is_some_and(|time| time.lights.is_some()) {
        log::info!(
            "mod {}: interior {}: the lights setting is not applied yet; they follow the hour",
            m.id,
            manifest.key
        );
    }
    Ok(ModInterior {
        owner: m.id.clone(),
        key: manifest.key,
        label: manifest.label,
        prompt: manifest.prompt,
        entry: manifest.entry,
        tile: first_tile + at,
        spawn: manifest.spawn,
        exit: manifest.exit,
        time: manifest.time,
        suspend: manifest.suspend,
    })
}
