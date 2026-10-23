//! The resident objects as tiles with their cull data, for the frame paths that sweep them with a per-tile shortcut
//! (`visibility.rs`). [`Residency::placed`] lists the same objects one by one.

use super::{Residency, TileState};
use crate::scenes::world::resident::{CullTile, TileCull};

impl Residency {
    /// The tiles [`Residency::placed`] draws from: the shared sets, then the resident tiles of the zone (and of the
    /// previous zone while the current one loads, and the map packs). The cull data of a tile is built here on the
    /// first call after it became resident.
    pub(in crate::scenes::world) fn cull_tiles(&self) -> impl Iterator<Item = CullTile<'_>> {
        // A resident map pack is drawn whatever the zone: no zone's list names it.
        let base = self.mod_base;
        let drawn = move |i: &usize| self.drawn.contains(i) || self.previous.contains(i) || *i >= base;
        let shared = std::iter::once((self.shared_placed.as_slice(), &self.shared));
        let tiles = self.tiles.iter().filter(move |(i, _)| drawn(i)).filter_map(|(_, state)| match state {
            TileState::Resident { resources, placed, .. } => Some((placed.as_slice(), &**resources)),
            TileState::Requested => None,
        });
        let collision = &self.collision;
        shared.chain(tiles).map(move |(placed, resources)| CullTile {
            placed,
            cull: resources.cull.get_or_init(|| TileCull::build(placed)),
            collision,
        })
    }
}
