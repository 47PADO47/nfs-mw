//! What the road view's sweep reads of one set's placements, kept apart from the large `ReflectionPlaced` records.
//!
//! A `ReflectionPlaced` is a few hundred bytes (matrix, four meshes, four local boxes), so a sweep over thousands of
//! them misses the cache on every object. [`RoadCull`] holds the two boxes the frustum test needs in a compact array,
//! plus the box of the whole set, so a set outside the view costs one test and an object outside it one small read.
use blackbox_scene::Aabb;

use super::geometry::{authored_slot, scenery_transform};
use crate::scenes::world::resident::ReflectionPlaced;
use crate::scenes::world::visibility::NOT_A_PROP;

/// The cull fields of one placement, from its place in the set's list.
#[derive(Clone, Copy)]
pub(super) struct RoadEntry {
    /// World bounds as placed.
    pub bounds: Aabb,
    /// World bounds of the authored mesh under the reflection matrix; the placed bounds when the mesh has none.
    pub reflected: Aabb,
    /// The prop world can move or hide it, so neither box says where it is drawn.
    pub dynamic: bool,
}

/// The compact cull data of one set of placements, built once the set is final.
#[derive(Default)]
pub(in crate::scenes::world) struct RoadCull {
    /// One entry per placement, in the list's order.
    pub(super) entries: Vec<RoadEntry>,
    /// The places of the dynamic entries, ascending. They are swept even when the rest of the set is culled.
    pub(super) dynamic: Vec<u32>,
    /// The box that holds both boxes of every static entry. `None` when the set has none.
    pub(super) bounds: Option<Aabb>,
}

impl RoadCull {
    pub(in crate::scenes::world) fn build(placed: &[ReflectionPlaced]) -> Self {
        let mut entries = Vec::with_capacity(placed.len());
        let mut dynamic = Vec::new();
        let mut bounds = Aabb::EMPTY;
        let mut statics = 0usize;
        for (place, p) in placed.iter().enumerate() {
            if p.prop_id != NOT_A_PROP {
                dynamic.push(place as u32);
                entries.push(RoadEntry { bounds: p.bounds, reflected: p.bounds, dynamic: true });
                continue;
            }
            let reflected = p.local_bounds[authored_slot(p.flags)]
                .map(|local| local.transformed(&scenery_transform(p.transform, p.flags)))
                .unwrap_or(p.bounds);
            statics += 1;
            bounds = bounds.union(&p.bounds).union(&reflected);
            entries.push(RoadEntry { bounds: p.bounds, reflected, dynamic: false });
        }
        match statics {
            0 => Self { entries, dynamic, bounds: None },
            _ => Self { entries, dynamic, bounds: Some(bounds) },
        }
    }
}
