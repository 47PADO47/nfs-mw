//! The rider's gear: every helmet, suit, pair of gloves and boots of RIDE 6's rider editor, converted once from the user's gear
//! catalogue (`tools/ride6-exporter/gear.py`) into the files the game reads one item at a time when the player picks it
//! (`docs/formats/ride6-rider-gear.md`, `docs/specs/ride6-rider-gear.md`).
//!
//! Input, the catalogue folder: `catalogue.json`, `meshes/<key>.r6rm`, `tex/<key>.r6tx`; every file is named by the hash of its
//! content, so a mesh or a picture two items share is one file. Output, `GEAR/` inside the mod's `files/RIDER/`: `gear.json`,
//! `meshes/<key>.bbsk` (one mesh, the first level of detail, sections by role: [`mesh`]) and `tex/<key>.r6tx` (the mip levels
//! up to a size: [`texture`]). Nothing is decoded and nothing is stored twice.

pub mod catalogue;
pub mod index;
pub mod mesh;
#[cfg(test)]
mod tests;
pub mod texture;

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use self::catalogue::Catalogue;
use self::index::Written;
use self::mesh::Role;

/// The folder of the gear inside the mod's `files/RIDER/`.
pub const FOLDER: &str = "GEAR";
/// The longest side of a picture the importer keeps by default, pixels.
pub const DEFAULT_MAX_TEXTURE: u32 = 512;

/// What one run wrote.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub bodies: usize,
    pub helmets: usize,
    pub suits: usize,
    pub gloves: usize,
    pub boots: usize,
    pub meshes: usize,
    pub textures: usize,
    pub mesh_bytes: u64,
    pub texture_bytes: u64,
    /// Items and files left out, each with the reason.
    pub left_out: Vec<String>,
}

/// What each mesh of the catalogue is a piece of, by key. A mesh two items use in different roles keeps the first and is
/// named in the second list.
fn roles(catalogue: &Catalogue) -> (BTreeMap<&str, Role>, Vec<String>) {
    let mut roles: BTreeMap<&str, Role> = BTreeMap::new();
    let mut conflicts = Vec::new();
    let uses = catalogue
        .bodies
        .iter()
        .map(|b| (b.mesh.as_str(), Role::Body))
        .chain(catalogue.helmets.iter().map(|h| (h.mesh.as_str(), Role::Helmet)))
        .chain(catalogue.helmets.iter().filter_map(|h| h.visor.as_ref()).map(|v| (v.mesh.as_str(), Role::Visor)))
        .chain(catalogue.boots.iter().map(|b| (b.mesh.as_str(), Role::Boots)));
    for (key, role) in uses {
        match roles.get(key) {
            Some(known) if *known != role => conflicts.push(format!("mesh {key} is used as {known:?} and as {role:?}")),
            Some(_) => {}
            None => {
                roles.insert(key, role);
            }
        }
    }
    (roles, conflicts)
}

/// The pictures the items wear.
fn picture_keys(catalogue: &Catalogue) -> Vec<&str> {
    let mut keys: Vec<&str> = catalogue
        .helmets
        .iter()
        .map(|h| h.color.as_str())
        .chain(catalogue.helmets.iter().filter_map(|h| h.visor.as_ref()).map(|v| v.color.as_str()))
        .chain(catalogue.suits.iter().map(|s| s.color.as_str()))
        .chain(catalogue.boots.iter().map(|b| b.color.as_str()))
        .collect();
    keys.sort_unstable();
    keys.dedup();
    keys
}

/// One mesh of the catalogue as a runtime file; its size.
fn write_mesh(catalogue_dir: &Path, catalogue: &Catalogue, key: &str, role: Role, output: &Path) -> Result<u64> {
    let file = catalogue.meshes.get(key).with_context(|| format!("mesh {key} is not in the catalogue"))?;
    let bytes = fs::read(catalogue_dir.join(&file.file)).with_context(|| format!("reading {}", file.file))?;
    let pack = mesh::pack_bytes(&bytes, role).with_context(|| file.file.clone())?;
    fs::write(output.join(index::mesh_path(key)), &pack).with_context(|| format!("writing mesh {key}"))?;
    Ok(pack.len() as u64)
}

/// One picture of the catalogue as a runtime file, cut to `max_texture`; its size.
fn write_texture(
    catalogue_dir: &Path,
    catalogue: &Catalogue,
    key: &str,
    max_texture: u32,
    output: &Path,
) -> Result<u64> {
    let file = catalogue.textures.get(key).with_context(|| format!("picture {key} is not in the catalogue"))?;
    let bytes = fs::read(catalogue_dir.join(&file.file)).with_context(|| format!("reading {}", file.file))?;
    let mut picture = texture::parse(&bytes).with_context(|| file.file.clone())?;
    texture::limit(&mut picture, max_texture);
    let out = texture::to_bytes(&picture);
    fs::write(output.join(index::texture_path(key)), &out).with_context(|| format!("writing picture {key}"))?;
    Ok(out.len() as u64)
}

/// Converts the catalogue in `catalogue_dir` into `output` (the `GEAR` folder). A mesh or picture that does not convert leaves
/// out the items that need it, which the summary names; a catalogue that cannot be read is an error.
pub fn import(catalogue_dir: &Path, output: &Path, max_texture: u32) -> Result<Summary> {
    let bytes = fs::read(catalogue_dir.join("catalogue.json")).context("reading catalogue.json")?;
    let catalogue = catalogue::parse(&bytes)?;
    fs::create_dir_all(output.join(index::MESHES)).context("creating the gear folder")?;
    fs::create_dir_all(output.join(index::TEXTURES)).context("creating the gear folder")?;
    let mut summary = Summary::default();
    let mut written = Written { meshes: HashSet::new(), textures: HashSet::new() };
    let (roles, conflicts) = roles(&catalogue);
    summary.left_out.extend(conflicts);
    for (key, role) in roles {
        match write_mesh(catalogue_dir, &catalogue, key, role, output) {
            Ok(size) => {
                written.meshes.insert(key.to_owned());
                summary.mesh_bytes += size;
            }
            Err(error) => summary.left_out.push(format!("mesh {key}: {error:#}")),
        }
    }
    for key in picture_keys(&catalogue) {
        match write_texture(catalogue_dir, &catalogue, key, max_texture, output) {
            Ok(size) => {
                written.textures.insert(key.to_owned());
                summary.texture_bytes += size;
            }
            Err(error) => summary.left_out.push(format!("picture {key}: {error:#}")),
        }
    }
    let (gear, left_out) = index::build(&catalogue, &written);
    summary.left_out.extend(left_out);
    let count = |name: &str| gear[name].as_array().map_or(0, Vec::len);
    summary.bodies = count("bodies");
    summary.helmets = count("helmets");
    summary.suits = count("suits");
    summary.gloves = count("gloves");
    summary.boots = count("boots");
    summary.meshes = written.meshes.len();
    summary.textures = written.textures.len();
    fs::write(output.join("gear.json"), serde_json::to_vec_pretty(&gear)?).context("writing gear.json")?;
    Ok(summary)
}
