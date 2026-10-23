//! Light metadata follows resident mesh ownership, including detached scenery parts.

use super::{Residency, TileState};
use blackbox_gfx::MeshHandle;
use nfsmw_data::world::lights::Marker;
use std::sync::Arc;

impl Residency {
    pub(in crate::scenes::world) fn light_sources(&self) -> impl Iterator<Item = (MeshHandle, Arc<[Marker]>)> + '_ {
        self.shared
            .flare_markers
            .iter()
            .chain(
                self.tiles
                    .values()
                    .filter_map(|tile| {
                        let TileState::Resident { resources, .. } = tile else { return None };
                        Some(resources.flare_markers.iter())
                    })
                    .flatten(),
            )
            .map(|(&mesh, markers)| (mesh, Arc::clone(markers)))
    }
}
