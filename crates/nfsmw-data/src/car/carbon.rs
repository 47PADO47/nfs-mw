//! Carbon hoods (docs/specs/car-assembly.md §6). A carbon hood is the same solid as its painted hood; the original draws it
//! through a second texture table in which the car's `<CAR>_SKIN1` paint placeholder shows `CARBONFIBRE`, and lights the
//! body groups with its `CARBONFIBER` light material instead of the paint's.

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;

use super::{CARSKIN, CarTables, Slots, slot};

/// The texture a carbon hood shows in place of the car's paint.
const CARBON: &str = "CARBONFIBRE";
/// The light material the original keeps beside the paint's for carbon parts (note the spelling, unlike the texture).
const CARBON_MATERIAL: &str = "CARBONFIBER";

/// When the hood slot holds a part whose `CARBONFIBRE` attribute is set, the hood's solids at `levels` show carbon fibre
/// where they would show the car's paint, lit as carbon fibre where they would be lit as paint (`CARSKIN`). The pooled solids are copied before they change, so a shared read stays as read.
pub(super) fn apply(
    t: &CarTables,
    slots: &Slots,
    levels: &[usize],
    base_model: &str,
    pool: &mut HashMap<u32, Arc<Solid>>,
) {
    let Some(hood) = slots[slot::HOOD] else { return };
    if t.parts.attribute(&hood, "CARBONFIBRE").is_none_or(|value| value == 0) {
        return;
    }
    let skin = bstring_hash(&format!("{base_model}_SKIN1"));
    let carbon = bstring_hash(CARBON);
    let carbon_material = bstring_hash(CARBON_MATERIAL);
    for &level in levels {
        let Some(hash) = t.parts.model_hash(&hood, level) else { continue };
        let Some(solid) = pool.get_mut(&hash) else { continue };
        let solid = Arc::make_mut(solid);
        for texture in &mut solid.texture_hashes {
            if *texture == skin {
                *texture = carbon;
            }
        }
        for material in &mut solid.light_material_hashes {
            if *material == CARSKIN {
                *material = carbon_material;
            }
        }
    }
}
