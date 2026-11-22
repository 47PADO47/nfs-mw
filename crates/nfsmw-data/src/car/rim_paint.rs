//! Isolate rim paint on eligible wheel solids so tires, brakes and body trim retain their materials.
use super::{CarModel, CarTables, Slots, slot};
use blackbox_carparts::LightMaterial;
use blackbox_hash::bstring_hash;

pub(super) fn apply(model: &mut CarModel, tables: &CarTables, slots: &Slots, custom: Option<[u8; 3]>) {
    if slots[slot::FRONT_WHEEL].is_none_or(|p| p.upgrade_level == 0) {
        return;
    }
    let Some(part) = slots[78] else {
        return;
    };
    let paint = super::Paint::of(tables, &part);
    let Some(material) = paint.light_material.and_then(|h| model.light_materials.get(&h)).cloned() else {
        return;
    };
    recolour(model, material, custom.unwrap_or(paint.rgb));
}

/// Draw the metal of every placed wheel with `material`, its diffuse colour scaled by `rgb`.
pub(super) fn recolour(model: &mut CarModel, mut material: LightMaterial, rgb: [u8; 3]) {
    let rgb = rgb.map(|v| f32::from(v) / 255.0);
    for (i, c) in rgb.into_iter().enumerate() {
        material.diffuse_min[i] *= c;
        material.diffuse_max[i] *= c;
    }
    // Keep the original wheel group's polish while applying the selected pigment and angular ramps.
    let finishes = ["MAGSILVER", "MAGCHROME", "MAGGUNMETAL"].map(|group| {
        let mut finish = material.clone();
        finish.name = format!("__GARAGE_RIM_PAINT_{}:{group}", material.name);
        finish.name_hash = bstring_hash(&finish.name);
        (bstring_hash(group), finish)
    });
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
            if let Some((_, finish)) = finishes.iter().find(|(original, _)| original == h) {
                *h = finish.name_hash;
            }
        }
    }
    model.light_materials.extend(finishes.into_iter().map(|(_, finish)| (finish.name_hash, finish)));
}
