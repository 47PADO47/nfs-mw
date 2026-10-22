use crate::scenes::world::space::to_physics;
use blackbox_collision::CollisionWorld;

use super::{Residency, TileState};

impl Residency {
    pub(super) fn wanted_resident(&self) -> bool {
        self.wanted.iter().all(|i| matches!(self.tiles.get(i), Some(TileState::Resident { .. })))
    }

    /// Keep driving while distant scenery loads. Actual local collision packs must be resident;
    /// zone names need not have a corresponding tile. Old collision/props stay until the new zone loads.
    pub fn simulation_ready(&self, [x, y]: [f32; 2], margin: f32) -> bool {
        if !self.shared_ready || !self.has_zone {
            return false;
        }
        let point = to_physics(glam::Vec3::new(x, y, 0.0));
        match self.collision.resident_near(point, margin) {
            Some(true) => true,
            // Every wanted tile is in, yet a collision pack the car's surroundings refer to is not. Nothing loads
            // that pack any more, so waiting for it would freeze the car for good. Drive on without it.
            Some(false) if self.wanted_resident() => {
                static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    log::warn!(
                        "collision near ({x:.0}, {y:.0}) refers to sections that are not loaded; driving on without them"
                    );
                }
                true
            }
            Some(false) => false,
            None => self.wanted_resident(),
        }
    }

    /// The collision of the tiles that are resident now.
    pub fn collision(&self) -> &CollisionWorld {
        &self.collision
    }

    /// Invalidates cached environment captures after streaming or a visibility-set change.
    pub fn visual_revision(&self) -> u64 {
        self.visual_revision
    }

    /// Whether the shared sets and every tile the current zone needs are resident.
    pub fn complete(&self) -> bool {
        self.shared_ready
            && self.has_zone
            && self.arrived.iter().all(|data| !self.wanted_set.contains(&data.index))
            && self.wanted_resident()
    }

    /// The current zone, e.g. `D14`.
    pub fn zone(&self) -> Option<String> {
        self.zones.name()
    }

    /// (resident tiles, loading tiles, shared sets ready).
    pub fn counts(&self) -> (usize, usize, bool) {
        let resident = self.tiles.values().filter(|s| matches!(s, TileState::Resident { .. })).count();
        (resident, self.tiles.len() - resident, self.shared_ready)
    }
}
