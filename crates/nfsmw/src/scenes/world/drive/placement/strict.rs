//! Exact placements (a race's start) never move the car to another road: the request waits for the ground at its
//! spot. Only after [`STRICT_WAIT`] does it look for ground from just above the spot, and only when that fails too
//! does it become an ordinary request that may search for a road.

use std::time::{Duration, Instant};

use blackbox_collision::CollisionWorld;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::super::{Drive, SpawnRequest};
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// How long a strict placement waits for the ground at its spot.
pub const STRICT_WAIT: Duration = Duration::from_secs(60);
/// How far above the spot the late ground probe starts, and how far below it goes.
const PROBE_ABOVE: f32 = 3.0;
const PROBE_BELOW: f32 = 40.0;

/// What a strict placement did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strict {
    Placed,
    /// The ground is not there yet: the request stays.
    Waiting,
    /// No ground at the spot after [`STRICT_WAIT`]: the request is now an ordinary one.
    Lost,
}

impl Drive {
    /// Puts the car at a strict request's spot, `spawn`.
    pub fn place_strict(&mut self, spawn: Spawn, collision: &CollisionWorld, surfaces: &SurfaceTable) -> Strict {
        if self.spawn(spawn, collision, surfaces) {
            return Strict::Placed;
        }
        let since = *self.strict_since.get_or_insert_with(Instant::now);
        if since.elapsed() < STRICT_WAIT {
            return Strict::Waiting;
        }
        self.strict_since = None;
        let p = spawn.position;
        let ground = space::ground_below(collision, p.x, p.y, p.z + PROBE_ABOVE, p.z - PROBE_BELOW);
        if let Some(lowered) = late_spot(spawn, ground)
            && self.spawn(lowered, collision, surfaces)
        {
            return Strict::Placed;
        }
        self.pending_speed = 0.0;
        self.request =
            Some(SpawnRequest { near: [p.x, p.y], heading: Some(spawn.heading), exact: None, strict: false });
        Strict::Lost
    }
}

/// The spot moved onto the ground found below it, if any.
fn late_spot(spawn: Spawn, ground: Option<f32>) -> Option<Spawn> {
    let height = ground?;
    Some(Spawn { position: Vec3::new(spawn.position.x, spawn.position.y, height), ..spawn })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_late_spot_keeps_the_place_and_heading_but_takes_the_ground() {
        let spawn = Spawn { position: Vec3::new(10.0, 20.0, 31.0), heading: 1.5 };
        let lowered = late_spot(spawn, Some(29.5)).unwrap();
        assert_eq!(lowered.position, Vec3::new(10.0, 20.0, 29.5));
        assert_eq!(lowered.heading, 1.5);
        assert_eq!(late_spot(spawn, None), None);
    }
}
