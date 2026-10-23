//! What the visibility sweep reads of a tile's placed objects, kept apart from the large [`Placed`] records.
//!
//! A [`Placed`] is about 250 bytes (matrix, LOD slots, shared fields), so a sweep over thousands of them misses the
//! cache on every object. [`TileCull`] holds the few fields the cull needs in a compact array, plus the tile's world
//! box, so a tile outside the view costs one test and an object outside it costs one 44 byte read. The `Placed` record
//! is read only for an object that survives.

use blackbox_collision::CollisionWorld;
use blackbox_scene::Aabb;
use blackbox_scenery::INSTANCE_TURNABLE;
use glam::Vec3;

use super::Placed;
use crate::scenes::world::visibility::{NOT_A_PROP, turned_pose};

/// The cull fields of one placed object, from its place in the tile's list.
#[derive(Clone, Copy)]
pub struct CullEntry {
    /// World bounds as placed (`Placed::bounds`).
    pub min: Vec3,
    pub max: Vec3,
    /// Where the object is drawn from: the translation of `Placed::transform`.
    pub position: Vec3,
    pub flags: u32,
    pub group: u16,
    pub original_shadow: bool,
    /// The prop world can move or hide it (`Placed::prop_id`), so neither its placed bounds nor its placed pose say
    /// where it is drawn. The sweep always asks the full record about it.
    pub dynamic: bool,
}

impl CullEntry {
    pub fn bounds(&self) -> Aabb {
        Aabb { min: self.min, max: self.max }
    }
}

/// The compact cull data of one tile (or of the shared sets), built from its `placed` list once it is resident.
#[derive(Default)]
pub struct TileCull {
    /// One entry per placed object, in the list's order.
    pub entries: Vec<CullEntry>,
    /// The places of the dynamic entries, ascending. They are swept even when the rest of the tile is culled.
    pub dynamic: Vec<u32>,
    /// The box that holds every static entry, including the swing of a turnable piece (`turned_pose`). `None` when the
    /// tile has no static entry.
    pub bounds: Option<Aabb>,
    /// The box that holds the origins of the static entries.
    pub origins: Option<Aabb>,
}

impl TileCull {
    pub fn build(placed: &[Placed]) -> Self {
        let mut entries = Vec::with_capacity(placed.len());
        let mut dynamic = Vec::new();
        let mut bounds = Aabb::EMPTY;
        let mut origins = Aabb::EMPTY;
        let mut statics = 0usize;
        for (place, p) in placed.iter().enumerate() {
            let position = p.transform.w_axis.truncate();
            let is_dynamic = p.prop_id != NOT_A_PROP;
            entries.push(CullEntry {
                min: p.bounds.min,
                max: p.bounds.max,
                position,
                flags: p.flags,
                group: p.group,
                original_shadow: p.original_shadow,
                dynamic: is_dynamic,
            });
            if is_dynamic {
                dynamic.push(place as u32);
                continue;
            }
            statics += 1;
            bounds = bounds.union(&p.bounds);
            if p.flags & INSTANCE_TURNABLE != 0 {
                bounds = bounds.union(&turned_pose(p.transform, &p.bounds).1);
            }
            origins.extend(position);
        }
        match statics {
            0 => Self { entries, dynamic, bounds: None, origins: None },
            _ => Self { entries, dynamic, bounds: Some(bounds), origins: Some(origins) },
        }
    }
}

/// One tile's placed objects with their cull data, and the collision world that says which scenery groups are in.
pub struct CullTile<'a> {
    pub placed: &'a [Placed],
    pub cull: &'a TileCull,
    pub collision: &'a CollisionWorld,
}

impl CullTile<'_> {
    /// Whether the residency lists an object of `group`: an ungrouped one always, a grouped one while its group is
    /// switched on (`Residency::group_shown`).
    pub fn group_shown(&self, group: u16) -> bool {
        group == 0 || self.collision.group_enabled(group)
    }
}
