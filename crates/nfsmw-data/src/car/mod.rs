//! Loading one car (geometry + textures) from the install.

mod parts;

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use blackbox_solid::Solid;
use blackbox_tpk::Texture;
use game_install::GameDir;

pub use parts::is_stock_part;

use crate::read_unwrapped;

/// A car's selected solids and the textures they use.
pub struct CarModel {
    pub name: String,
    pub solids: Vec<Solid>,
    /// Textures used by `solids`, by name hash.
    pub textures: HashMap<u32, Texture>,
}

/// Packs searched for car textures, in priority order. `{car}` is the car folder.
const TEXTURE_FILES: &[&str] = &["CARS/{car}/TEXTURES.BIN", "CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC"];

/// Car folders that have a model.
pub fn list(dir: &GameDir) -> Vec<String> {
    dir.subdirectories("CARS").into_iter().filter(|car| dir.exists(&format!("CARS/{car}/GEOMETRY.BIN"))).collect()
}

/// Load `car` at level of detail `lod` ('A' = highest); `all_parts` keeps every kit and decal.
pub fn load(dir: &GameDir, car: &str, lod: char, all_parts: bool) -> Result<CarModel> {
    let geometry_path = format!("CARS/{car}/GEOMETRY.BIN");
    if !dir.exists(&geometry_path) {
        bail!("no car named {car:?} (try `nfsmw list-cars`)");
    }
    let geometry = read_unwrapped(dir, &geometry_path)?;
    let all = blackbox_solid::read_solids(&geometry).with_context(|| format!("parsing {geometry_path}"))?;
    let total = all.len();
    let solids: Vec<Solid> = all.into_iter().filter(|s| is_stock_part(&s.name, lod, all_parts)).collect();
    if solids.is_empty() {
        bail!("{car} has no solids at LOD {lod}");
    }
    let wanted: HashSet<u32> =
        solids.iter().flat_map(|s| s.groups.iter().filter_map(|g| g.diffuse_texture(s))).collect();
    let textures = load_textures(dir, car, &wanted)?;
    log::info!(
        "{car}: {} of {total} solids at LOD {lod}; textures: {} referenced, {} found",
        solids.len(),
        wanted.len(),
        textures.len()
    );
    Ok(CarModel { name: car.to_owned(), solids, textures })
}

fn load_textures(dir: &GameDir, car: &str, wanted: &HashSet<u32>) -> Result<HashMap<u32, Texture>> {
    let mut textures = HashMap::new();
    for pattern in TEXTURE_FILES {
        let rel = pattern.replace("{car}", car);
        if !dir.exists(&rel) {
            continue;
        }
        let file = read_unwrapped(dir, &rel)?;
        for pack in blackbox_tpk::read_texture_packs(&file).with_context(|| format!("parsing textures in {rel}"))? {
            for t in pack.textures.into_iter().filter(|t| wanted.contains(&t.name_hash)) {
                textures.entry(t.name_hash).or_insert(t);
            }
            for (hash, why) in pack.failed.iter().filter(|(h, _)| wanted.contains(h)) {
                log::debug!("{rel}: texture 0x{hash:08X}: {why}");
            }
        }
    }
    Ok(textures)
}
