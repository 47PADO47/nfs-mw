//! The map packs of the mods (docs/specs/mod-world-content.md, sections 2.4 to 2.6). Each `scenery/<pack>.toml` is read and
//! checked against the install's streaming index. Its model file is `files/` of the mod, parsed like an install section,
//! and the placements are built into a scenery section once the file is loaded (`assemble`). A pack that fails a check is
//! logged with its reason and left out; the game goes on.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use blackbox_modding::content::{Collision, Orientation, Place, SCENERY_FOLDER, SceneryManifest, parse_scenery};
use blackbox_scenery::build::{self, Placement, yaw_rows};
use blackbox_solid::Solid;
use blackbox_streaming::StreamingSection;
use game_install::{GameDir, Provider};
use nfsmw_data::world::{SectionData, parse_section};

use super::{ModDir, toml_files};

/// The game's rule for an object's exclude flags when the manifest leaves them out: drawn in every main view.
const DEFAULT_EXCLUDE: u32 = 0x60;
/// The flag of `boost`: +10 px in the level-of-detail choice (`docs/specs/scenery-lod.md`).
const BOOST_FLAG: u32 = 0x0200_0000;

/// A pack the residency can load: its streaming entry, its file and its manifest.
#[derive(Debug, Clone)]
pub struct MapPack {
    /// The mod that declares the pack.
    pub owner: String,
    /// The pack key: the manifest's file name.
    pub key: String,
    /// The streaming entry: its number and anchor, and the file as the whole section (offset 0).
    pub section: StreamingSection,
    /// The pack file on disk.
    pub path: PathBuf,
    pub manifest: SceneryManifest,
}

impl MapPack {
    /// Whether the pack's static collision goes into the world: a placement asks for it.
    pub fn has_collision(&self) -> bool {
        self.manifest.places.iter().any(|place| place.collision == Collision::Static)
    }
}

/// The packs of `mods`, in load order and then file-name order. `installed` holds the section numbers of the install's
/// index, which a pack may not take.
pub fn packs(dir: &GameDir, mods: &[ModDir], installed: &HashSet<i16>) -> Vec<MapPack> {
    let mut taken = installed.clone();
    let mut out = Vec::new();
    for m in mods {
        for path in toml_files(&m.dir.join(SCENERY_FOLDER)) {
            match read_pack(dir, m, &path, &taken) {
                Ok(pack) => {
                    log::info!("mod {}: map pack {} at section {}", m.id, pack.key, pack.section.number);
                    taken.insert(pack.section.number);
                    out.push(pack);
                }
                Err(reason) => log::warn!("mod {}: map pack {} is not added: {reason}", m.id, path.display()),
            }
        }
    }
    out
}

/// Read and check one pack. The error is the reason, for the log.
fn read_pack(dir: &GameDir, m: &ModDir, path: &Path, taken: &HashSet<i16>) -> Result<MapPack, String> {
    let stem = path.file_stem().and_then(|stem| stem.to_str()).ok_or("the file name is not text")?;
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let manifest = parse_scenery(stem, &text).map_err(|error| error.to_string())?;
    let number = i16::try_from(manifest.section).map_err(|_| "the section number does not fit".to_owned())?;
    free_section(number)?;
    if taken.contains(&number) {
        return Err(format!("section {number} is already taken by the install or another pack"));
    }
    let file = pack_path(dir, m, &manifest.file)?;
    let size = std::fs::metadata(&file).map_err(|error| error.to_string())?.len();
    let bytes = std::fs::read(&file).map_err(|error| error.to_string())?;
    let data = parse_section(0, &bytes).map_err(|error| format!("the model file does not parse: {error:#}"))?;
    check_models(&manifest, &data)?;
    let section = entry(&manifest, number, size)?;
    Ok(MapPack { owner: m.id.clone(), key: manifest.key.clone(), section, path: file, manifest })
}

/// The bands a mod may take: `K`, `N`, `U` and `W` (no section in the install), and 2700 and up. Inside a band the last two
/// digits are 0 to 39 or 80 to 99, so the remap of the tile numbers (`docs/specs/mod-world-content.md`, 1.2) never folds a
/// pack onto another number.
fn free_section(number: i16) -> Result<(), String> {
    let band = matches!(number, 1100..=1199 | 1400..=1499 | 2100..=2199 | 2300..=2399) || number >= 2700;
    if !band {
        return Err(format!("section {number} is not in a free band (K, N, U, W, or 2700 and up)"));
    }
    if !matches!(number % 100, 0..=39 | 80..=99) {
        return Err(format!("section {number}: its last two digits must be 0 to 39 or 80 to 99"));
    }
    Ok(())
}

/// The pack file, which must be in this mod's own `files/` folder (not another layer's). Its path on disk.
fn pack_path(dir: &GameDir, m: &ModDir, file: &str) -> Result<PathBuf, String> {
    let own = matches!(dir.provider(file), Some(Provider::Overlay(id)) if id == m.id);
    if !own {
        return Err(format!("{file} is not in the mod's files/ folder"));
    }
    dir.resolve(file).map(Path::to_path_buf).ok_or_else(|| format!("{file} cannot be read"))
}

/// Every model a placement names must be a solid of the pack file.
fn check_models(manifest: &SceneryManifest, data: &SectionData) -> Result<(), String> {
    for place in &manifest.places {
        let names = [Some(place.model.as_str()), place.coarse.as_deref(), place.mirror.as_deref()];
        for name in names.into_iter().flatten() {
            if solid(data, name).is_none() {
                return Err(format!("model {name} is not in the pack file"));
            }
        }
    }
    Ok(())
}

/// The streaming entry of a pack: anchor centre and radius, and the file size as the whole section.
fn entry(manifest: &SceneryManifest, number: i16, size: u64) -> Result<StreamingSection, String> {
    let size = u32::try_from(size).map_err(|_| "the pack file is too large".to_owned())?;
    let [x, y, _, radius] = manifest.anchor;
    Ok(StreamingSection {
        name: manifest.key.to_ascii_uppercase(),
        number,
        file_type: 0,
        file_offset: 0,
        size,
        compressed_size: size,
        perm_size: 0,
        priority: 0,
        center: [x, y],
        radius,
        checksum: 0,
    })
}

/// Build a loaded pack's placed objects into a scenery section of its number, and keep its static collision only when a
/// placement asks for it. The collision packs take the pack's number, so each is told apart from the others.
pub fn assemble(data: &mut SectionData, pack: &MapPack) -> Result<(), String> {
    let number = u32::from(pack.section.number.unsigned_abs());
    let placements = pack.manifest.places.iter().map(|place| placement(data, place)).collect::<Result<Vec<_>, _>>()?;
    let section = build::section(number, &placements).map_err(|error| format!("{error:?}"))?;
    data.scenery.push(section);
    if !pack.has_collision() {
        data.collision.clear();
    }
    for collision in &mut data.collision {
        collision.section = number;
    }
    Ok(())
}

/// One placement as the builder takes it: the solids' hashes, the model's bounds, and the turn.
fn placement(data: &SectionData, place: &Place) -> Result<Placement, String> {
    let hash_of = |name: &str| {
        solid(data, name).map(|s| s.name_hash).ok_or_else(|| format!("model {name} is not in the pack file"))
    };
    let model = hash_of(&place.model)?;
    let mirror = place.mirror.as_deref().map(hash_of).transpose()?.unwrap_or(model);
    let coarse = place.coarse.as_deref().map(hash_of).transpose()?.unwrap_or(0);
    let model_solid = solid(data, &place.model).ok_or("the model is missing")?;
    let rows = match place.orientation {
        Orientation::Yaw(degrees) => yaw_rows(degrees, place.scale),
        Orientation::Rows(rows) => rows.map(|row| row.map(|v| v * place.scale)),
    };
    let boost = u32::from(place.boost) * BOOST_FLAG;
    let exclude = place.exclude.unwrap_or(DEFAULT_EXCLUDE) | boost;
    Ok(Placement {
        name: place.model.clone(),
        solids: [model, mirror, coarse, 0],
        local_min: model_solid.bounds_min,
        local_max: model_solid.bounds_max,
        radius: radius_of(model_solid),
        position: place.pos,
        rows,
        exclude_flags: exclude,
    })
}

/// The solid of the pack file called `name`, ignoring case.
fn solid<'a>(data: &'a SectionData, name: &str) -> Option<&'a Solid> {
    data.solids.iter().find(|solid| solid.name.eq_ignore_ascii_case(name))
}

/// The distance from the model's origin to the farthest corner of its local box.
fn radius_of(solid: &Solid) -> f32 {
    (0..8usize)
        .map(|corner| [0, 1, 2].map(|axis| [solid.bounds_min[axis], solid.bounds_max[axis]][(corner >> axis) & 1]))
        .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
        .fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::free_section;

    #[test]
    fn the_free_bands_and_the_numbers_above_2700_are_accepted() {
        for number in [1100, 1139, 1180, 1199, 1400, 2100, 2300, 2399, 2700, 2739, 2780, 32000] {
            assert!(free_section(number).is_ok(), "{number} is free");
        }
    }

    #[test]
    fn the_install_sections_and_the_remap_ranges_are_refused() {
        for number in [141, 1000, 1200, 1240, 1140, 1179, 2600, 2650] {
            assert!(free_section(number).is_err(), "{number} is not free");
        }
    }
}
