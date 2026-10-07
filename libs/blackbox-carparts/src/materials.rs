//! `LightMaterials`: shading constants of car materials (`eLightMaterial`).

use blackbox_chunk::ids;

use crate::bytes::{cstr, f32_at, u32_at};
use crate::layout::CarDataLayout;

/// One light material. The "min" values apply at grazing angles and the "max" values facing the
/// viewer (docs/specs/car-assembly.md §8; the mapping is unconfirmed).
#[derive(Debug, Clone, PartialEq)]
pub struct LightMaterial {
    pub name: String,
    pub name_hash: u32,
    pub version: u32,
    pub diffuse_min_scale: f32,
    pub diffuse_min: [f32; 3],
    pub diffuse_max_scale: f32,
    pub diffuse_max: [f32; 3],
    pub diffuse_min_alpha: f32,
    pub diffuse_max_alpha: f32,
    pub specular_power: f32,
    pub specular_min_scale: f32,
    pub specular_min: [f32; 3],
    pub specular_max_scale: f32,
    pub specular_max: [f32; 3],
    pub envmap_power: f32,
    pub envmap_min_scale: f32,
    pub envmap_min: [f32; 3],
    pub envmap_max_scale: f32,
    pub envmap_max: [f32; 3],
    pub metallic_scale: f32,
    pub specular_hot_spot: f32,
}

/// Every `LightMaterials` chunk of `data` (one material each).
pub fn read_light_materials(data: &[u8], layout: &CarDataLayout) -> Vec<LightMaterial> {
    let l = &layout.light_material;
    let len = l.values + 30 * 4;
    blackbox_chunk::find_all(data, ids::LIGHT_MATERIALS)
        .into_iter()
        .filter(|c| c.payload.len() >= len)
        .map(|c| {
            let p = c.payload;
            let mut values = (0..30).map(|i| f32_at(p, l.values + i * 4));
            let mut f = || values.next().unwrap_or(0.0);
            LightMaterial {
                name: cstr(p, l.name, l.name_len),
                name_hash: u32_at(p, l.name_hash),
                version: u32_at(p, l.version),
                diffuse_min_scale: f(),
                diffuse_min: [f(), f(), f()],
                diffuse_max_scale: f(),
                diffuse_max: [f(), f(), f()],
                diffuse_min_alpha: f(),
                diffuse_max_alpha: f(),
                specular_power: f(),
                specular_min_scale: f(),
                specular_min: [f(), f(), f()],
                specular_max_scale: f(),
                specular_max: [f(), f(), f()],
                envmap_power: f(),
                envmap_min_scale: f(),
                envmap_min: [f(), f(), f()],
                envmap_max_scale: f(),
                envmap_max: [f(), f(), f()],
                metallic_scale: f(),
                specular_hot_spot: f(),
            }
        })
        .collect()
}
