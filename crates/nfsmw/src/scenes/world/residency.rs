//! Which sections are loaded and drawn: the shared sets once, map tiles by the
//! camera's zone (`docs/specs/visible-sections.md`).

use std::collections::{HashMap, HashSet};

use blackbox_collision::{CollisionWorld, Grid};
use blackbox_render::Renderer;
use blackbox_streaming::{StreamingSection, VisibleSections};
use nfsmw_data::world::{GlobalTextures, SectionData, Streamer, StreamerEvent};

use super::resident::{Placed, SectionResources, place};
use super::zone::Zones;

/// Requests in flight at once.
const MAX_IN_FLIGHT: usize = 8;
/// Tiles uploaded per frame (uploads run on the render thread).
const UPLOADS_PER_FRAME: usize = 2;

enum TileState {
    Requested,
    Resident {
        resources: SectionResources,
        placed: Vec<Placed>,
        /// Sections of the collision packs this tile put into the collision world.
        packs: Vec<u32>,
    },
}

pub struct Residency {
    sections: Vec<StreamingSection>,
    streamer: Streamer,
    shared_total: usize,
    /// Shared sets wait here until all have arrived: their textures and models cross-reference.
    shared_pending: Vec<SectionData>,
    /// Textures from the global packs, uploaded with the shared sets.
    globals: GlobalTextures,
    pub shared: SectionResources,
    pub shared_placed: Vec<Placed>,
    shared_ready: bool,
    tiles: HashMap<usize, TileState>,
    arrived: Vec<SectionData>,
    zones: Zones,
    /// Tiles the current zone loads, drawn ones first.
    wanted: Vec<usize>,
    /// Tiles the current zone draws.
    drawn: HashSet<usize>,
    /// Tiles the previous zone drew, still drawn until the current zone's tiles are all resident,
    /// so crossing a zone border does not open holes while tiles load.
    previous: HashSet<usize>,
    pub unresolved: usize,
    /// The collision of the resident tiles; physics queries run on it (`docs/formats/collision.md`).
    collision: CollisionWorld,
}

impl Residency {
    pub fn new(
        sections: Vec<StreamingSection>,
        visible: VisibleSections,
        streamer: Streamer,
        globals: GlobalTextures,
        collision_grid: Option<Grid>,
    ) -> Self {
        // Every shared set stays loaded: V/X/Y hold models and textures only, Z0 the sky.
        let shared: Vec<usize> = (0..sections.len()).filter(|&i| !sections[i].is_spatial()).collect();
        for &i in &shared {
            streamer.request(i);
        }
        Self {
            shared_total: shared.len(),
            zones: Zones::new(visible, &sections),
            sections,
            streamer,
            shared_pending: Vec::new(),
            globals,
            shared: SectionResources::default(),
            shared_placed: Vec::new(),
            shared_ready: false,
            tiles: HashMap::new(),
            arrived: Vec::new(),
            wanted: Vec::new(),
            drawn: HashSet::new(),
            previous: HashSet::new(),
            unresolved: 0,
            collision: CollisionWorld::new(collision_grid),
        }
    }

    /// Request, upload and drop sections for a camera at map position (x, y).
    pub fn update(&mut self, renderer: &mut Renderer, x: f32, y: f32) {
        self.receive();
        if !self.shared_ready {
            self.finish_shared(renderer);
            return;
        }
        if self.zones.update(x, y) {
            self.switch_zone(x, y);
        }
        self.upload_arrived(renderer);
        self.unload_unwanted(renderer);
        self.request_wanted();
    }

    fn switch_zone(&mut self, x: f32, y: f32) {
        self.previous = std::mem::replace(&mut self.drawn, self.zones.drawn());
        self.wanted = self.zones.wanted();
        // Nearest first within the drawn tiles, then within the rest.
        let key = |i: usize| (!self.drawn.contains(&i), self.sections[i].distance_to(x, y));
        self.wanted.sort_by(|&a, &b| {
            let (ka, kb) = (key(a), key(b));
            ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        });
        // Requests the new zone doesn't need are dropped; they are ignored when they arrive.
        let wanted: HashSet<usize> = self.wanted.iter().copied().collect();
        self.tiles.retain(|i, s| wanted.contains(i) || matches!(s, TileState::Resident { .. }));
        log::debug!("zone {:?}: {} tiles drawn, {} loaded", self.zones.name(), self.drawn.len(), self.wanted.len());
    }

    fn receive(&mut self) {
        while let Some(event) = self.streamer.poll() {
            match event {
                StreamerEvent::Loaded(data) if !self.sections[data.index].is_spatial() => {
                    self.shared_pending.push(*data)
                }
                StreamerEvent::Loaded(data) => self.arrived.push(*data),
                StreamerEvent::Failed { index, error } => {
                    log::error!("section {}: {error}", self.sections[index].name);
                    if self.sections[index].is_spatial() {
                        self.tiles.insert(
                            index,
                            TileState::Resident { resources: Default::default(), placed: vec![], packs: vec![] },
                        );
                    } else {
                        self.shared_total -= 1;
                    }
                }
            }
        }
    }

    fn finish_shared(&mut self, renderer: &mut Renderer) {
        if self.shared_pending.len() < self.shared_total {
            return;
        }
        let pending = std::mem::take(&mut self.shared_pending);
        let mut shared = SectionResources::default();
        for data in &pending {
            shared.upload_textures(renderer, data);
        }
        let globals = std::mem::take(&mut self.globals);
        shared.upload_texture_list(renderer, &globals.textures);
        shared.anims.extend(globals.anims);
        shared.anims.extend(pending.iter().flat_map(|d| d.anims.iter().cloned()));
        let empty = SectionResources::default();
        for data in &pending {
            shared.upload_meshes(renderer, data, &empty);
        }
        for data in &pending {
            let (placed, unresolved) = place(data, &shared, &empty);
            self.shared_placed.extend(placed);
            self.unresolved += unresolved;
        }
        log::info!("shared sets resident: {} models, {} textures", shared.meshes.len(), shared.materials.len());
        self.shared = shared;
        self.shared_ready = true;
    }

    fn upload_arrived(&mut self, renderer: &mut Renderer) {
        let take = self.arrived.len().min(UPLOADS_PER_FRAME);
        for data in self.arrived.drain(..take).collect::<Vec<_>>() {
            if !matches!(self.tiles.get(&data.index), Some(TileState::Requested)) {
                continue; // no longer wanted
            }
            let mut resources = SectionResources { anims: data.anims.clone(), ..Default::default() };
            resources.upload_textures(renderer, &data);
            resources.upload_meshes(renderer, &data, &self.shared);
            let (placed, unresolved) = place(&data, &resources, &self.shared);
            self.unresolved += unresolved;
            let packs = data.collision.iter().map(|p| p.section).collect();
            for pack in data.collision {
                self.collision.insert(pack);
            }
            self.tiles.insert(data.index, TileState::Resident { resources, placed, packs });
        }
    }

    /// Free the tiles the zone no longer needs, once everything it needs is resident (so the
    /// old zone's tiles stay usable while the new ones load).
    fn unload_unwanted(&mut self, renderer: &mut Renderer) {
        if !self.wanted_resident() {
            return;
        }
        self.previous.clear();
        let wanted: HashSet<usize> = self.wanted.iter().copied().collect();
        let unwanted: Vec<usize> = self.tiles.keys().copied().filter(|i| !wanted.contains(i)).collect();
        for i in unwanted {
            if let Some(TileState::Resident { resources, packs, .. }) = self.tiles.remove(&i) {
                resources.release(renderer);
                for section in packs {
                    self.collision.remove(section);
                }
            }
        }
    }

    fn request_wanted(&mut self) {
        let in_flight = self.tiles.values().filter(|s| matches!(s, TileState::Requested)).count();
        let free = MAX_IN_FLIGHT.saturating_sub(in_flight);
        let missing: Vec<usize> =
            self.wanted.iter().copied().filter(|i| !self.tiles.contains_key(i)).take(free).collect();
        for i in missing {
            self.tiles.insert(i, TileState::Requested);
            self.streamer.request(i);
        }
    }

    fn wanted_resident(&self) -> bool {
        self.wanted.iter().all(|i| matches!(self.tiles.get(i), Some(TileState::Resident { .. })))
    }

    /// Advance every resident texture animation.
    pub fn animate(&self, renderer: &mut Renderer, seconds: f32) {
        let empty = SectionResources::default();
        self.shared.animate(renderer, seconds, &empty);
        for tile in self.tiles.values() {
            if let TileState::Resident { resources, .. } = tile {
                resources.animate(renderer, seconds, &self.shared);
            }
        }
    }

    /// Placed instances to draw: the shared sets' and those of the resident tiles the zone draws
    /// (plus the previous zone's while the current one loads).
    pub fn placed(&self) -> impl Iterator<Item = &Placed> {
        let drawn = |i: &usize| self.drawn.contains(i) || self.previous.contains(i);
        let tiles = self.tiles.iter().filter(move |(i, _)| drawn(i)).flat_map(|(_, s)| match s {
            TileState::Resident { placed, .. } => placed.as_slice(),
            TileState::Requested => &[],
        });
        self.shared_placed.iter().chain(tiles)
    }

    /// Whether the shared sets and every tile the current zone needs are resident.
    pub fn complete(&self) -> bool {
        self.shared_ready && self.zones.name().is_some() && self.arrived.is_empty() && self.wanted_resident()
    }

    /// The collision of the tiles that are resident now.
    pub fn collision(&self) -> &CollisionWorld {
        &self.collision
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
