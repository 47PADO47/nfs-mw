//! `Vehicles`: the player's car, read and moved (catalogue: docs/modding/api-world.md#vehicles-and-the-player).

use super::ffi::vehicles::{self as host_vehicles, VehiclesFns};
use super::{ModResult, VehicleId};

/// The player's car at one moment, in render space (x forward, y left, z up; metres and seconds).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VehicleState {
    /// -1 reverse, 0 neutral, 1 and up the forward gears.
    pub gear: i32,
    /// Position, metres.
    pub position: [f32; 3],
    /// Linear velocity, metres per second.
    pub velocity: [f32; 3],
    /// Heading in radians, from +x towards +y.
    pub heading: f32,
    /// Speed along the car, metres per second (negative in reverse).
    pub speed: f32,
    /// Engine speed, revolutions per minute.
    pub rpm: f32,
    /// Nitrous left, 0 to 1.
    pub nos: f32,
    /// Damage, 0 undamaged to 1.
    pub damage: f32,
}

/// The `bb.vehicles` interface of the host, as a mod uses it. Obtained from `Ctx::vehicles`.
#[derive(Clone, Copy)]
pub struct Vehicles {
    fns: VehiclesFns,
}

impl Vehicles {
    pub(crate) fn new(fns: VehiclesFns) -> Self {
        Vehicles { fns }
    }

    /// The player's car now.
    pub fn player(&self) -> ModResult<VehicleState> {
        host_vehicles::state(&self.fns)
    }

    /// Move the player's car to the road nearest `position`, facing `heading`. The move is made after the mods run.
    pub fn teleport_player(&self, position: [f32; 3], heading: f32) -> ModResult<()> {
        host_vehicles::teleport(&self.fns, position, heading)
    }

    /// Clear the player's car's damage and punctures.
    pub fn repair_player(&self) -> ModResult<()> {
        host_vehicles::repair(&self.fns)
    }

    /// Spawn a traffic car near `position`. The host answers `Unsupported` in 1.0 (see the catalogue).
    pub fn spawn_traffic(&self, position: [f32; 3], heading: f32) -> ModResult<VehicleId> {
        host_vehicles::spawn_traffic(&self.fns, position, heading).map(VehicleId::from_raw)
    }
}
