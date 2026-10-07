//! Loading one car (geometry + textures) from the install into CPU-side data.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use nfsmw_geometry::Solid;
use nfsmw_install::GameDir;
use nfsmw_texture::{AlphaUsage, Texture};

/// A car ready to hand to the renderer.
pub struct CarModel {
    pub name: String,
    pub solids: Vec<Solid>,
    /// Textures used by `solids`, by name hash.
    pub textures: HashMap<u32, Texture>,
    pub bounds_min: glam::Vec3,
    pub bounds_max: glam::Vec3,
}

#[derive(Debug, Default)]
pub struct LoadStats {
    pub solids_total: usize,
    pub textures_referenced: usize,
    pub textures_missing: usize,
    /// Textures that exist but could not be decoded (e.g. an unsupported compressor).
    pub textures_failed: usize,
}

/// Packs searched for car textures, in priority order. `{car}` is the car folder.
const TEXTURE_FILES: &[&str] = &["CARS/{car}/TEXTURES.BIN", "CARS/TEXTURES.BIN", "GLOBAL/GLOBALB.LZC"];

fn read_unwrapped(dir: &GameDir, rel: &str) -> Result<Vec<u8>> {
    let raw = dir.read(rel).with_context(|| format!("reading {rel}"))?;
    Ok(nfsmw_compress::unwrap(&raw).with_context(|| format!("decompressing {rel}"))?.into_owned())
}

/// Whether a solid belongs to the stock car at `lod`.
fn is_stock_part(name: &str, lod: char, all_parts: bool) -> bool {
    let suffix = format!("_{lod}");
    if !name.ends_with(&suffix) {
        return false;
    }
    if all_parts {
        return true;
    }
    // Decals need vinyl textures we don't load yet; KIT00 is the stock body kit.
    !name.contains("_DECAL_") && (!name.contains("_KIT") || name.contains("_KIT00_"))
}

pub fn load(dir: &GameDir, car: &str, lod: char, all_parts: bool) -> Result<CarModel> {
    let geometry_path = format!("CARS/{car}/GEOMETRY.BIN");
    if !dir.exists(&geometry_path) {
        bail!("no car named {car:?} (try `nfsmw list-cars`)");
    }
    let geometry = read_unwrapped(dir, &geometry_path)?;
    let all = nfsmw_geometry::read_solids(&geometry).with_context(|| format!("parsing {geometry_path}"))?;
    let mut stats = LoadStats { solids_total: all.len(), ..Default::default() };
    let solids: Vec<Solid> = all.into_iter().filter(|s| is_stock_part(&s.name, lod, all_parts)).collect();
    if solids.is_empty() {
        bail!("{car} has no solids at LOD {lod}");
    }

    let wanted: std::collections::HashSet<u32> =
        solids.iter().flat_map(|s| s.groups.iter().filter_map(|g| g.diffuse_texture(s))).collect();
    stats.textures_referenced = wanted.len();

    let mut textures = HashMap::new();
    let mut failed = std::collections::HashSet::new();
    for pattern in TEXTURE_FILES {
        let rel = pattern.replace("{car}", car);
        if !dir.exists(&rel) {
            continue;
        }
        let file = read_unwrapped(dir, &rel)?;
        for pack in nfsmw_texture::read_texture_packs(&file).with_context(|| format!("parsing textures in {rel}"))? {
            for t in pack.textures {
                if wanted.contains(&t.name_hash) {
                    textures.entry(t.name_hash).or_insert(t);
                }
            }
            for (hash, why) in pack.failed {
                if wanted.contains(&hash) && failed.insert(hash) {
                    log::debug!("{rel}: texture 0x{hash:08X}: {why}");
                }
            }
        }
    }
    stats.textures_failed = failed.iter().filter(|h| !textures.contains_key(h)).count();
    stats.textures_missing = wanted.iter().filter(|h| !textures.contains_key(h)).count() - stats.textures_failed;

    let mut bounds_min = glam::Vec3::splat(f32::MAX);
    let mut bounds_max = glam::Vec3::splat(f32::MIN);
    for v in solids.iter().flat_map(|s| &s.vertices) {
        let p = glam::Vec3::from(v.position);
        bounds_min = bounds_min.min(p);
        bounds_max = bounds_max.max(p);
    }

    log::info!(
        "{car}: {} of {} solids at LOD {lod}; textures: {} referenced, {} loaded, {} failed to decode, {} not found",
        solids.len(),
        stats.solids_total,
        stats.textures_referenced,
        textures.len(),
        stats.textures_failed,
        stats.textures_missing,
    );
    Ok(CarModel { name: car.to_owned(), solids, textures, bounds_min, bounds_max })
}

/// How a texture's alpha should be drawn.
pub fn blend_mode(t: Option<&Texture>) -> nfsmw_render::BlendMode {
    match t.map(|t| t.alpha_usage) {
        Some(AlphaUsage::Modulated) => nfsmw_render::BlendMode::AlphaBlend,
        Some(AlphaUsage::PunchThrough) => nfsmw_render::BlendMode::AlphaTest,
        _ => nfsmw_render::BlendMode::Opaque,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_part_filter() {
        assert!(is_stock_part("BMWM3GTR_BASE_A", 'A', false));
        assert!(is_stock_part("BMWM3GTR_KIT00_BODY_A", 'A', false));
        assert!(!is_stock_part("BMWM3GTR_KIT01_BODY_A", 'A', false));
        assert!(!is_stock_part("BMWM3GTR_KIT00_BODY_B", 'A', false));
        assert!(!is_stock_part("BMWM3GTR_KIT00_DECAL_LEFT_DOOR_RECT_MEDIUM_A", 'A', false));
        assert!(is_stock_part("BMWM3GTR_KIT00_DECAL_LEFT_DOOR_RECT_MEDIUM_A", 'A', true));
    }
}
