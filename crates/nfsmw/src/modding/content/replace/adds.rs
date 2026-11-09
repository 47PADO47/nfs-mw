//! The objects a replacement set places in the world (`[[add]]`): map packs the game makes when a world opens.
//!
//! A set's objects are scenery placements of solids in the set's geometry file, so each one is a placement of a map pack
//! (`content::scenery`, `docs/modding/api-content-worlds.md`). The placements are grouped by the 512 m cell they are in, and each
//! group becomes a pack: its file is the set's geometry file (on disk), its section number the next free one of the mods' band
//! (2700 and up), its anchor a circle round the group, so a pack loads while the camera is near it and costs nothing elsewhere. The
//! textures of the models are the set's, uploaded once into the shared sets. The packs are made when a world opens from the
//! replacements in effect then: a choice made in a drive shows at the next world.

use std::collections::{BTreeMap, HashSet};

use blackbox_modding::content::{Collision, Place};
use game_install::GameDir;

use crate::modding::content::scenery::{MapPack, next_free_section, synthetic_pack};

/// The side of the square cell a pack covers, metres.
const CELL: f32 = 512.0;
/// What an anchor holds beyond the objects, metres.
const MARGIN: f32 = 30.0;

/// The packs of the objects the replacements in effect place, with section numbers not in `taken`. A set whose geometry is not a plain
/// file of the install (a lazy file), an object whose model is not in the geometry and an object that asks for prop collision are left
/// out with a warning.
pub fn packs(dir: &GameDir, taken: &HashSet<i16>) -> Vec<MapPack> {
    let Some(active) = super::active() else { return Vec::new() };
    let mut taken = taken.clone();
    let mut next = 0;
    let mut out = Vec::new();
    for added in active.objects() {
        let Some(file) = dir.resolve(&added.geometry).map(std::path::Path::to_path_buf) else {
            log::warn!(
                "replacements: {}:{}: {} is not a file, its objects are not placed",
                added.owner,
                added.set,
                added.geometry
            );
            continue;
        };
        let mut cells: BTreeMap<(i32, i32), Vec<Place>> = BTreeMap::new();
        for place in &added.places {
            if !added.models.contains(&place.model.to_ascii_uppercase()) {
                log::warn!(
                    "replacements: {}:{}: the object {} is not in {}",
                    added.owner,
                    added.set,
                    place.model,
                    added.geometry
                );
                continue;
            }
            if place.collision == Collision::Prop {
                log::warn!(
                    "replacements: {}:{}: prop collision is not offered, {} is not placed",
                    added.owner,
                    added.set,
                    place.model
                );
                continue;
            }
            let cell = ((place.pos[0] / CELL).floor() as i32, (place.pos[1] / CELL).floor() as i32);
            cells.entry(cell).or_default().push(place.clone());
        }
        for ((cx, cy), places) in cells {
            let Some((number, after)) = next_free_section(next, &taken) else {
                log::warn!("replacements: {}:{}: no free section number is left", added.owner, added.set);
                return out;
            };
            next = after;
            let key = format!("add-{}-{cx}-{cy}", added.set);
            let made =
                synthetic_pack(&added.owner, &key, number, file.clone(), &added.geometry, anchor_of(&places), places);
            match made {
                Ok(pack) => {
                    log::info!(
                        "replacements: {}:{}: {} objects in the pack {key} at section {number}",
                        added.owner,
                        added.set,
                        pack.manifest.places.len()
                    );
                    taken.insert(number);
                    out.push(pack);
                }
                Err(reason) => {
                    log::warn!("replacements: {}:{}: the pack {key} is not made: {reason}", added.owner, added.set)
                }
            }
        }
    }
    out
}

/// The circle that holds `places`: centre (x, y, z) and radius.
fn anchor_of(places: &[Place]) -> [f32; 4] {
    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    for place in places {
        for axis in 0..3 {
            low[axis] = low[axis].min(place.pos[axis]);
            high[axis] = high[axis].max(place.pos[axis]);
        }
    }
    let centre = [0.5 * (low[0] + high[0]), 0.5 * (low[1] + high[1]), 0.5 * (low[2] + high[2])];
    let (dx, dy) = (high[0] - low[0], high[1] - low[1]);
    [centre[0], centre[1], centre[2], 0.5 * dx.hypot(dy) + MARGIN]
}

#[cfg(test)]
#[path = "adds_tests.rs"]
mod tests;
