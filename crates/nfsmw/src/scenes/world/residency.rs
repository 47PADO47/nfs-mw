//! Which sections are loaded: the shared sets once, map tiles by distance.

use std::collections::HashMap;

use blackbox_render::Renderer;
use blackbox_streaming::StreamingSection;
use nfsmw_data::world::{SectionData, Streamer, StreamerEvent};

use super::resident::{Placed, SectionResources, place};

/// Requests in flight at once.
const MAX_IN_FLIGHT: usize = 8;
/// Tiles uploaded per frame (uploads run on the render thread).
const UPLOADS_PER_FRAME: usize = 2;

enum TileState {
    Requested,
    Resident { resources: SectionResources, placed: Vec<Placed> },
}

pub struct Residency {
    sections: Vec<StreamingSection>,
    streamer: Streamer,
    shared_total: usize,
    /// Shared sets wait here until all have arrived: their textures and models cross-reference.
    shared_pending: Vec<SectionData>,
    pub shared: SectionResources,
    pub shared_placed: Vec<Placed>,
    shared_ready: bool,
    tiles: HashMap<usize, TileState>,
    arrived: Vec<SectionData>,
    pub load_radius: f32,
    pub unresolved: usize,
}

impl Residency {
    pub fn new(sections: Vec<StreamingSection>, streamer: Streamer, load_radius: f32) -> Self {
        let shared: Vec<usize> = (0..sections.len()).filter(|&i| !sections[i].is_spatial()).collect();
        for &i in &shared {
            streamer.request(i);
        }
        Self {
            shared_total: shared.len(),
            sections,
            streamer,
            shared_pending: Vec::new(),
            shared: SectionResources::default(),
            shared_placed: Vec::new(),
            shared_ready: false,
            tiles: HashMap::new(),
            arrived: Vec::new(),
            load_radius,
            unresolved: 0,
        }
    }

    /// Request, upload and drop sections for a camera at map position (x, y).
    pub fn update(&mut self, renderer: &mut Renderer, x: f32, y: f32) {
        self.receive();
        if !self.shared_ready {
            self.finish_shared(renderer);
            return;
        }
        self.upload_arrived(renderer);
        self.unload_far(renderer, x, y);
        self.request_near(x, y);
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
                        self.tiles.insert(index, TileState::Resident { resources: Default::default(), placed: vec![] });
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
                continue; // unloaded while in flight
            }
            let mut resources = SectionResources::default();
            resources.upload_textures(renderer, &data);
            resources.upload_meshes(renderer, &data, &self.shared);
            let (placed, unresolved) = place(&data, &resources, &self.shared);
            self.unresolved += unresolved;
            self.tiles.insert(data.index, TileState::Resident { resources, placed });
        }
    }

    fn unload_far(&mut self, renderer: &mut Renderer, x: f32, y: f32) {
        let keep = self.load_radius * 1.25 + 50.0;
        let far: Vec<usize> =
            self.tiles.keys().copied().filter(|&i| self.sections[i].distance_to(x, y) > keep).collect();
        for i in far {
            if let Some(TileState::Resident { resources, .. }) = self.tiles.remove(&i) {
                resources.release(renderer);
            }
        }
    }

    fn request_near(&mut self, x: f32, y: f32) {
        let in_flight = self.tiles.values().filter(|s| matches!(s, TileState::Requested)).count();
        let mut wanted: Vec<(f32, usize)> = (0..self.sections.len())
            .filter(|&i| self.sections[i].is_spatial() && !self.tiles.contains_key(&i))
            .map(|i| (self.sections[i].distance_to(x, y), i))
            .filter(|&(d, _)| d <= self.load_radius)
            .collect();
        wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, i) in wanted.into_iter().take(MAX_IN_FLIGHT.saturating_sub(in_flight)) {
            self.tiles.insert(i, TileState::Requested);
            self.streamer.request(i);
        }
    }

    /// Placed instances of every resident section.
    pub fn placed(&self) -> impl Iterator<Item = &Placed> {
        let tiles = self.tiles.values().flat_map(|s| match s {
            TileState::Resident { placed, .. } => placed.as_slice(),
            TileState::Requested => &[],
        });
        self.shared_placed.iter().chain(tiles)
    }

    /// Whether the shared sets and every tile within the load radius of (x, y) are resident.
    pub fn complete_at(&self, x: f32, y: f32) -> bool {
        self.shared_ready
            && self.arrived.is_empty()
            && (0..self.sections.len())
                .filter(|&i| self.sections[i].is_spatial() && self.sections[i].distance_to(x, y) <= self.load_radius)
                .all(|i| matches!(self.tiles.get(&i), Some(TileState::Resident { .. })))
    }

    /// (resident tiles, loading tiles, shared sets ready).
    pub fn counts(&self) -> (usize, usize, bool) {
        let resident = self.tiles.values().filter(|s| matches!(s, TileState::Resident { .. })).count();
        (resident, self.tiles.len() - resident, self.shared_ready)
    }
}
