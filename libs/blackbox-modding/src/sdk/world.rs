//! `World`: the track and the district, the rain, and the traffic density (catalogue: docs/modding/api-world.md#world-state).

use super::ModResult;
use super::ffi::world::{self as host_world, WorldFns};

/// The `bb.world` interface of the host, as a mod uses it. Obtained from `Ctx::world`.
#[derive(Clone, Copy)]
pub struct World {
    fns: WorldFns,
}

impl World {
    pub(crate) fn new(fns: WorldFns) -> Self {
        World { fns }
    }

    /// The name of the track. `NotReady` while no world is loaded.
    pub fn track(&self) -> ModResult<String> {
        host_world::track(&self.fns)
    }

    /// The name of the district at the player's position.
    pub fn section(&self) -> ModResult<String> {
        host_world::section(&self.fns)
    }

    /// Keep it raining (`always`), or leave it to the weather, at `amount` (0 to 1).
    pub fn set_rain(&self, always: bool, amount: f32) -> ModResult<()> {
        host_world::set_rain(&self.fns, always, amount)
    }

    /// Keep `cars` computer-driven cars on the road, `cop_share` percent of them patrol cops (at most 100).
    pub fn set_traffic(&self, cars: u32, cop_share: u32) -> ModResult<()> {
        host_world::set_traffic(&self.fns, cars, cop_share)
    }
}
