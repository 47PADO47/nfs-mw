//! The map packs of the mods in the residency (docs/specs/mod-world-content.md, 2.5 and 2.9). A pack is a tile that no zone
//! lists: it loads while the camera is inside its anchor plus a margin, or while an interior holds it (`pin`). Each pack
//! has its own streamer, reading its own file, and its indices come after the install's (`mod_base`).

use blackbox_collision::CollisionPack;
use nfsmw_data::world::{SectionData, StreamerEvent};

use super::{Residency, TileState};
use crate::modding::content::scenery::assemble;

/// How far outside its anchor a pack stays loaded, metres (the spec's design value).
pub const MOD_MARGIN: f32 = 150.0;

impl Residency {
    /// Whether the residency index `tile` belongs to a mod's map pack.
    pub(super) fn is_mod(&self, tile: usize) -> bool {
        tile >= self.mod_base
    }

    /// Whether the map pack at residency index `tile` is loaded and drawn.
    pub fn pack_resident(&self, tile: usize) -> bool {
        matches!(self.tiles.get(&tile), Some(TileState::Resident { .. }))
    }

    /// Keep (`on`) or release the map pack at `tile` for an interior. A pinned pack loads whatever the camera does.
    pub fn pin(&mut self, tile: usize, on: bool) {
        match on {
            true => self.pinned.insert(tile),
            false => self.pinned.remove(&tile),
        };
    }

    /// Which map packs the camera at (x, y) wants: those whose anchor is within [`MOD_MARGIN`], and the pinned ones.
    pub(super) fn update_mod_wanted(&mut self, x: f32, y: f32) {
        if self.mod_packs.is_empty() {
            return;
        }
        let near = (0..self.mod_packs.len())
            .map(|k| self.mod_base + k)
            .filter(|&tile| self.sections[tile].distance_to(x, y) <= MOD_MARGIN);
        let mut wanted = std::mem::take(&mut self.mod_wanted);
        wanted.clear();
        wanted.extend(near);
        wanted.extend(self.pinned.iter().copied());
        wanted.sort_unstable();
        wanted.dedup();
        self.mod_wanted = wanted;
    }

    /// Request the wanted map packs that are not in flight or loaded yet.
    pub(super) fn request_mod(&mut self) {
        for k in 0..self.mod_wanted.len() {
            let tile = self.mod_wanted[k];
            if self.tiles.contains_key(&tile) {
                continue;
            }
            self.tiles.insert(tile, TileState::Requested);
            self.mod_streamers[tile - self.mod_base].request(0);
        }
    }

    /// Take in the results of the map packs' streamers. A parsed pack becomes its scenery and its collision here.
    pub(super) fn receive_mod(&mut self) {
        for k in 0..self.mod_streamers.len() {
            let tile = self.mod_base + k;
            let events: Vec<StreamerEvent> = std::iter::from_fn(|| self.mod_streamers[k].poll()).collect();
            for event in events {
                match event {
                    StreamerEvent::Collision { .. } => {}
                    StreamerEvent::Loaded(mut data) => {
                        data.index = tile;
                        self.assemble_mod(&mut data, k);
                        self.handle(StreamerEvent::Loaded(data));
                    }
                    StreamerEvent::Failed { error, .. } => self.handle(StreamerEvent::Failed { index: tile, error }),
                }
            }
        }
    }

    fn assemble_mod(&self, data: &mut SectionData, k: usize) {
        if let Err(reason) = assemble(data, &self.mod_packs[k]) {
            log::warn!("map pack {}: its objects are not placed: {reason}", self.mod_packs[k].key);
            data.scenery.clear();
            data.collision.clear();
        }
    }

    /// Insert a pack's collision: the grid knows the install's packs, a mod's pack is tested on every query.
    pub(super) fn insert_collision(&mut self, pack: CollisionPack, tile: usize) {
        if self.is_mod(tile) {
            return self.collision.insert_ungridded(pack);
        }
        self.collision.insert(pack);
    }

    /// Remove the collision a tile put in, the way it went in.
    pub(super) fn remove_collision(&mut self, section: u32, tile: usize) {
        if self.is_mod(tile) {
            self.collision.remove_ungridded(section);
            return;
        }
        if !self.traffic_owns_collision(section) {
            self.collision.remove(section);
        }
    }
}
