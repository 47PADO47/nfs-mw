//! Light metadata follows resident mesh ownership, including detached scenery parts.

use super::{Residency, TileState};
use crate::scenes::world::local_lights::FixturePlace;
use crate::scenes::world::resident::Placed;
use blackbox_gfx::MeshHandle;
use nfsmw_data::world::lights::Marker;
use std::sync::Arc;

impl Residency {
    /// The placed objects with local-light fixtures in the drawn tiles (as [`Residency::cull_tiles`] lists them),
    /// skipping switched-off scenery groups. Each tile's fixture index is built once, on its first scan.
    pub(in crate::scenes::world) fn fixture_placements(&self) -> impl Iterator<Item = (&Placed, &FixturePlace)> {
        let base = self.mod_base;
        let drawn = move |i: &usize| self.drawn.contains(i) || self.previous.contains(i) || *i >= base;
        let shared = &self.shared;
        let tiles = self.tiles.iter().filter(move |(i, _)| drawn(i)).filter_map(|(_, state)| match state {
            TileState::Resident { resources, placed, .. } => Some((placed.as_slice(), &**resources)),
            TileState::Requested => None,
        });
        std::iter::once((self.shared_placed.as_slice(), shared))
            .chain(tiles)
            .flat_map(move |(placed, resources)| {
                let places = resources.fixture_places.get_or_init(|| {
                    FixturePlace::index(placed, |mesh| {
                        resources.local_fixtures.get(&mesh).or_else(|| shared.local_fixtures.get(&mesh)).cloned()
                    })
                });
                places.iter().filter_map(|place| Some((placed.get(place.place as usize)?, place)))
            })
            .filter(|(placed, _)| self.group_shown(placed))
    }

    /// Every resident fixture mesh with its bulbs; the live path reads the per-tile index instead.
    #[cfg(test)]
    pub(in crate::scenes::world) fn local_fixtures(
        &self,
    ) -> impl Iterator<Item = (MeshHandle, &[super::super::local_lights::Fixture])> + '_ {
        self.shared
            .local_fixtures
            .iter()
            .chain(
                self.tiles
                    .values()
                    .filter_map(|tile| {
                        let TileState::Resident { resources, .. } = tile else { return None };
                        Some(resources.local_fixtures.iter())
                    })
                    .flatten(),
            )
            .map(|(&mesh, fixture)| (mesh, &fixture[..]))
    }

    pub(in crate::scenes::world) fn emitter_placements(
        &self,
    ) -> impl Iterator<Item = ((usize, usize), &nfsmw_data::world::emitter_placements::Placement)> + '_ {
        self.shared.emitters.iter().enumerate().map(|(i, p)| ((usize::MAX, i), p)).chain(self.tiles.iter().flat_map(
            |(&tile, state)| {
                let placements: &[_] = match state {
                    TileState::Resident { resources, .. } => &resources.emitters,
                    _ => &[],
                };
                placements.iter().enumerate().map(move |(i, p)| ((tile, i), p))
            },
        ))
    }
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
