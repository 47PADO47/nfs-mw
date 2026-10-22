//! Isolate rim paint on eligible wheel solids so tires, brakes and body trim retain their materials.
use super::{CarModel, CarTables, Slots, slot};
use blackbox_hash::bstring_hash;

pub(super) fn apply(model: &mut CarModel, tables: &CarTables, slots: &Slots, custom: Option<[u8; 3]>) {
    if slots[slot::FRONT_WHEEL].is_none_or(|p| p.upgrade_level == 0) {
        return;
    }
    let Some(part) = slots[78] else {
        return;
    };
    let paint = super::Paint::of(tables, &part);
    let Some(mut material) = paint.light_material.and_then(|h| model.light_materials.get(&h)).cloned() else {
        return;
    };
    let rgb = custom.unwrap_or(paint.rgb).map(|v| f32::from(v) / 255.0);
    for (i, c) in rgb.into_iter().enumerate() {
        material.diffuse_min[i] *= c;
        material.diffuse_max[i] *= c;
    }
    let hash = bstring_hash("__GARAGE_RIM_PAINT");
    material.name = format!("__GARAGE_RIM_PAINT_{}", material.name);
    material.name_hash = hash;
    let eligible = ["MAGSILVER", "MAGCHROME", "MAGGUNMETAL"].map(bstring_hash);
    let wheels: std::collections::HashSet<_> = model
        .placements
        .iter()
        .chain(model.driving_lods.levels.iter().flat_map(|l| &l.placements))
        .filter(|p| [slot::FRONT_WHEEL, slot::REAR_WHEEL].contains(&p.slot))
        .map(|p| p.solid)
        .collect();
    for (id, solid) in &mut model.solids {
        if !wheels.contains(id) {
            continue;
        }
        for h in &mut solid.light_material_hashes {
            if eligible.contains(h) {
                *h = hash;
            }
        }
    }
    model.light_materials.insert(hash, material);
}
