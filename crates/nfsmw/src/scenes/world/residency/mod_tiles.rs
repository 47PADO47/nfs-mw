//! The map packs of the mods in the residency (docs/specs/mod-world-content.md, 2.5 and 2.9). A pack is a tile that no zone
//! lists: it loads while the camera is inside its anchor plus a margin, or while an interior holds it (`pin`). Each pack
//! has its own streamer, reading its own file, and its indices come after the install's (`mod_base`).

use std::sync::Arc;

use blackbox_collision::CollisionPack;
use game_install::GameDir;
use nfsmw_data::world::{SectionData, SectionReader, Streamer, StreamerEvent};

use super::{Residency, TileState};
use crate::modding::content::scenery::{MapPack, PackSource, assemble};

/// Loader threads of the streamer the installed packs share (the pieces of a world made when they are asked for).
const INSTALLED_THREADS: usize = 3;

/// The streamers of a world's map packs, the route to each pack (streamer, section in it) and the pack of each section. A pack
/// that is a file has a streamer of its own, reading it from offset 0 as entry 0. The packs read through the install
/// (`PackSource::Installed`: a world made of many pieces, some made when asked for) share one streamer, whose sections are those
/// packs in order, so a world of hundreds of pieces does not start hundreds of threads.
pub(super) fn spawn_streamers(packs: &[MapPack]) -> (Vec<Streamer>, Vec<(usize, usize)>, Vec<Vec<usize>>) {
    let mut streamers = Vec::new();
    let mut owners: Vec<Vec<usize>> = Vec::new();
    let mut routes = vec![(0, 0); packs.len()];
    let mut installed: Vec<(usize, &GameDir, &str)> = Vec::new();
    for (k, pack) in packs.iter().enumerate() {
        match &pack.source {
            PackSource::File(path) => {
                routes[k] = (streamers.len(), 0);
                owners.push(vec![k]);
                streamers.push(Streamer::spawn(path.clone(), vec![pack.section.clone()], 1));
            }
            PackSource::Installed { dir, file } => installed.push((k, dir, file)),
        }
    }
    if installed.is_empty() {
        return (streamers, routes, owners);
    }
    let shared = streamers.len();
    let sections = installed.iter().map(|&(k, _, _)| packs[k].section.clone()).collect();
    let files: Vec<(GameDir, String)> =
        installed.iter().map(|&(_, dir, file)| (dir.clone(), file.to_owned())).collect();
    let reader: Arc<SectionReader> = Arc::new(move |section| {
        let (dir, file) = files.get(section).ok_or_else(|| format!("no piece {section}"))?;
        dir.read(file).map_err(|error| format!("{file}: {error}"))
    });
    for (section, &(k, _, _)) in installed.iter().enumerate() {
        routes[k] = (shared, section);
    }
    owners.push(installed.iter().map(|&(k, _, _)| k).collect());
    streamers.push(Streamer::spawn_made(reader, sections, INSTALLED_THREADS));
    (streamers, routes, owners)
}

/// How far outside its anchor a pack stays loaded, metres (the spec's design value).
pub const MOD_MARGIN: f32 = 150.0;

impl Residency {
    /// Makes the collision grid the dynamic one of a second world (`Grid::empty` was passed to `adopt`): its map packs' instances
    /// are listed in it when they arrive.
    pub fn set_dynamic_grid(&mut self, on: bool) {
        self.dynamic_grid = on;
    }

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
            let (streamer, section) = self.mod_routes[tile - self.mod_base];
            self.mod_streamers[streamer].request(section);
        }
    }

    /// Take in the results of the map packs' streamers. A parsed pack becomes its scenery and its collision here.
    pub(super) fn receive_mod(&mut self) {
        for streamer in 0..self.mod_streamers.len() {
            let events: Vec<StreamerEvent> = std::iter::from_fn(|| self.mod_streamers[streamer].poll()).collect();
            for event in events {
                match event {
                    StreamerEvent::Collision { .. } => {}
                    StreamerEvent::Loaded(mut data) => {
                        let Some(&k) = self.mod_owners[streamer].get(data.index) else { continue };
                        data.index = self.mod_base + k;
                        self.assemble_mod(&mut data, k);
                        self.handle(StreamerEvent::Loaded(data));
                    }
                    StreamerEvent::Failed { index, error } => {
                        let Some(&k) = self.mod_owners[streamer].get(index) else { continue };
                        log::warn!("map pack {}: {error}", self.mod_packs[k].key);
                        self.handle(StreamerEvent::Failed { index: self.mod_base + k, error });
                    }
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
        if self.is_mod(tile) && self.dynamic_grid {
            return self.collision.insert_into_grid(pack);
        }
        if self.is_mod(tile) {
            return self.collision.insert_ungridded(pack);
        }
        self.collision.insert(pack);
    }

    /// Remove the collision a tile put in, the way it went in.
    pub(super) fn remove_collision(&mut self, section: u32, tile: usize) {
        if self.is_mod(tile) && self.dynamic_grid {
            self.collision.remove_from_grid(section);
            return;
        }
        if self.is_mod(tile) {
            self.collision.remove_ungridded(section);
            return;
        }
        if !self.traffic_owns_collision(section) {
            self.collision.remove(section);
        }
    }
}
