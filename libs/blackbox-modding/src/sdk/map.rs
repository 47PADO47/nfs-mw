//! `Map`: add, move and remove blips on the map (catalogue: docs/modding/api-world.md#map-blips-and-the-world-map).

use super::ffi::map::{self as host_map, MapFns};
use super::{BlipId, ModResult};

/// The `bb.map` interface of the host, as a mod uses it. Obtained from `Ctx::map`.
#[derive(Clone, Copy)]
pub struct Map {
    fns: MapFns,
}

impl Map {
    pub(crate) fn new(fns: MapFns) -> Self {
        Map { fns }
    }

    /// Add a blip of a registered style at `position` (render space, metres). Returns its handle.
    pub fn add(&self, style: &str, position: [f32; 3], active: bool) -> ModResult<BlipId> {
        host_map::add(&self.fns, style, position, active)
    }

    /// Replace a blip's style, position and state.
    pub fn update(&self, id: BlipId, style: &str, position: [f32; 3], active: bool) -> ModResult<()> {
        host_map::update(&self.fns, id, style, position, active)
    }

    /// Remove a blip this mod added.
    pub fn remove(&self, id: BlipId) -> ModResult<()> {
        host_map::remove(&self.fns, id)
    }
}
