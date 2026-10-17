//! Which sections are loaded and drawn: the shared sets once, map tiles by the
//! camera's zone (`docs/specs/visible-sections.md`).

use std::collections::{HashMap, HashSet};

use blackbox_collision::{CollisionWorld, Grid};
use blackbox_gfx::RenderBackend;
use blackbox_streaming::{StreamingSection, VisibleSections};
use nfsmw_data::world::{GlobalTextures, PropCatalog, SectionData, Streamer, StreamerEvent};

use super::keys;
use super::props::PropWorld;
use super::resident::{Placed, ReflectionPlaced, SectionResources, place, place_reflections};
use super::zone::Zones;
mod shadow_support;

#[cfg(test)]
mod installed_tests;
#[cfg(test)]
mod shadow_tests;
#[cfg(test)]
mod tests;

/// Requests in flight at once.
const MAX_IN_FLIGHT: usize = 8;
/// Tiles uploaded per frame (uploads run on the render thread).
const UPLOADS_PER_FRAME: usize = 2;

enum TileState {
    Requested,
    Resident {
        resources: Box<SectionResources>,
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
    pub sky: super::sky::Sky,
    pub shared_placed: Vec<Placed>,
    shared_ready: bool,
    tiles: HashMap<usize, TileState>,
    arrived: Vec<SectionData>,
    zones: Zones,
    /// Tiles the current zone loads, drawn ones first.
    wanted: Vec<usize>,
    /// Additional light support, independent of colour visibility and readiness.
    shadow_wanted: Vec<usize>,
    shadow_range: f32,
    shadow_direction: Option<[f32; 2]>,
    shadow_extent: [f32; 2],
    shadow_candidates: Vec<(usize, f32)>,
    shadow_eligible: usize,
    /// Tiles the current zone draws.
    drawn: HashSet<usize>,
    /// Tiles the previous zone drew, still drawn until the current zone's tiles are all resident,
    /// so crossing a zone border does not open holes while tiles load.
    previous: HashSet<usize>,
    pub unresolved: usize,
    /// The collision of the resident tiles; physics queries run on it (`docs/formats/collision.md`).
    collision: CollisionWorld,
    /// The props of the resident tiles, and which of them are knocked over.
    props: PropWorld,
    catalog: PropCatalog,
    /// Changes when the scenery available to a view changes, including while the clock is paused.
    visual_revision: u64,
}

impl Residency {
    /// Release new registered materials on scene replacement as well as tile retirement.
    pub(super) fn release_normal_materials(&mut self, renderer: &mut dyn RenderBackend) {
        self.shared.release_normal_materials(renderer);
        for tile in self.tiles.values_mut() {
            if let TileState::Resident { resources, .. } = tile {
                resources.release_normal_materials(renderer);
            }
        }
    }

    pub fn new(
        sections: Vec<StreamingSection>,
        visible: VisibleSections,
        streamer: Streamer,
        globals: GlobalTextures,
        collision_grid: Option<Grid>,
        catalog: PropCatalog,
    ) -> Self {
        // Every shared set stays loaded, including the sky geometry in X0.
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
            sky: super::sky::Sky::default(),
            shared_placed: Vec::new(),
            shared_ready: false,
            tiles: HashMap::new(),
            arrived: Vec::new(),
            wanted: Vec::new(),
            shadow_wanted: Vec::new(),
            shadow_range: 0.0,
            shadow_direction: None,
            shadow_extent: [1.0, 0.0],
            shadow_candidates: Vec::new(),
            shadow_eligible: 0,
            drawn: HashSet::new(),
            previous: HashSet::new(),
            unresolved: 0,
            collision: CollisionWorld::new(collision_grid),
            props: PropWorld::default(),
            catalog,
            visual_revision: 0,
        }
    }

    /// Request, upload and drop sections for a camera at map position (x, y).
    pub fn update(&mut self, renderer: &mut dyn RenderBackend, x: f32, y: f32) {
        self.receive();
        if !self.shared_ready {
            self.finish_shared(renderer);
            return;
        }
        let changed = self.zones.update(x, y);
        if changed {
            self.switch_zone(renderer, x, y);
        }
        if !changed {
            self.update_shadow_support(renderer, [x, y]);
        }
        self.upload_arrived(renderer);
        self.unload_unwanted(renderer);
        self.request_wanted();
    }

    fn switch_zone(&mut self, renderer: &dyn RenderBackend, x: f32, y: f32) {
        self.visual_revision = self.visual_revision.wrapping_add(1);
        self.previous = std::mem::replace(&mut self.drawn, self.zones.drawn());
        self.wanted = self.zones.wanted();
        // Nearest first within the drawn tiles, then within the rest.
        let key = |i: usize| (!self.drawn.contains(&i), self.sections[i].distance_to(x, y));
        self.wanted.sort_by(|&a, &b| {
            let (ka, kb) = (key(a), key(b));
            ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        });
        self.update_shadow_support(renderer, [x, y]);
        // Requests the new zone doesn't need are dropped; they are ignored when they arrive.
        let wanted: HashSet<usize> = self.wanted.iter().copied().collect();
        let retained_requests: Vec<_> = self
            .shadow_wanted
            .iter()
            .copied()
            .filter(|i| !wanted.contains(i) && matches!(self.tiles.get(i), Some(TileState::Requested)))
            .take(shadow_support::MAX_SHADOW_IN_FLIGHT)
            .collect();
        self.tiles.retain(|i, s| {
            wanted.contains(i) || retained_requests.contains(i) || matches!(s, TileState::Resident { .. })
        });
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

    fn finish_shared(&mut self, renderer: &mut dyn RenderBackend) {
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
        // Additional global shared models; a stream solid of the same name wins.
        shared.upload_solid_list(renderer, &globals.solids, &empty, true);
        self.sky.release(renderer);
        self.sky = super::sky::Sky::upload(
            renderer,
            pending.iter().flat_map(|data| &data.solids).chain(&globals.solids),
            &shared,
        );
        for data in &pending {
            let (placed, unresolved) = place(data, &shared, &empty, &self.catalog);
            let mut reflected = place_reflections(data, &shared, &empty);
            for reflected in &mut reflected {
                reflected.player_index = reflected.player_index.map(|i| i + self.shared_placed.len());
            }
            shared.reflection_placed.extend(reflected);
            self.shared_placed.extend(placed);
            self.unresolved += unresolved;
        }
        for (index, placed) in self.shared_placed.iter_mut().enumerate() {
            placed.key = keys::shared(index);
        }
        self.props.add_tile(usize::MAX, &mut self.shared_placed);
        for (index, reflected) in shared.reflection_placed.iter_mut().enumerate() {
            reflected.bind_player(&self.shared_placed, keys::shared(index | (1 << 31)));
        }
        log::info!("shared sets resident: {} models, {} textures", shared.meshes.len(), shared.materials.len());
        self.shared = shared;
        self.shared_ready = true;
        self.visual_revision = self.visual_revision.wrapping_add(1);
    }

    fn upload_arrived(&mut self, renderer: &mut dyn RenderBackend) {
        self.arrived.sort_unstable_by_key(|data| !self.wanted.contains(&data.index));
        let take = self.arrived.len().min(UPLOADS_PER_FRAME);
        for data in self.arrived.drain(..take).collect::<Vec<_>>() {
            if !matches!(self.tiles.get(&data.index), Some(TileState::Requested)) {
                continue; // no longer wanted
            }
            let mut resources = SectionResources { anims: data.anims.clone(), ..Default::default() };
            resources.upload_textures(renderer, &data);
            resources.upload_meshes(renderer, &data, &self.shared);
            let (mut placed, unresolved) = place(&data, &resources, &self.shared, &self.catalog);
            for (index, object) in placed.iter_mut().enumerate() {
                object.key = keys::tile(data.index, index);
            }
            self.props.add_tile(data.index, &mut placed);
            resources.reflection_placed = place_reflections(&data, &resources, &self.shared);
            for (index, reflected) in resources.reflection_placed.iter_mut().enumerate() {
                reflected.bind_player(&placed, keys::tile(data.index, index | (1 << 31)));
            }
            self.unresolved += unresolved;
            let packs = data.collision.iter().map(|p| p.section).collect();
            for pack in data.collision {
                self.collision.insert(pack);
            }
            self.tiles.insert(data.index, TileState::Resident { resources: Box::new(resources), placed, packs });
            self.visual_revision = self.visual_revision.wrapping_add(1);
        }
    }

    /// Free the tiles the zone no longer needs, once everything it needs is resident (so the
    /// old zone's tiles stay usable while the new ones load).
    fn unload_unwanted(&mut self, renderer: &mut dyn RenderBackend) {
        if !self.wanted_resident() {
            return;
        }
        if !self.previous.is_empty() {
            self.visual_revision = self.visual_revision.wrapping_add(1);
            self.previous.clear();
        }
        let wanted: HashSet<usize> = self.wanted.iter().copied().collect();
        // Only the bounded support selection can retain extra geometry. Previous visits
        // must not accumulate every tile inside the larger hysteresis region.
        let unwanted: Vec<usize> = self
            .tiles
            .keys()
            .copied()
            .filter(|i| !wanted.contains(i))
            .filter(|i| !self.shadow_wanted.contains(i))
            .collect();
        for i in unwanted {
            if let Some(TileState::Resident { resources, packs, .. }) = self.tiles.remove(&i) {
                self.visual_revision = self.visual_revision.wrapping_add(1);
                resources.release(renderer);
                self.props.remove_tile(i);
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
        self.request_shadow_support();
    }

    fn wanted_resident(&self) -> bool {
        self.wanted.iter().all(|i| matches!(self.tiles.get(i), Some(TileState::Resident { .. })))
    }

    /// Advance every resident texture animation.
    pub fn animate(&self, renderer: &mut dyn RenderBackend, seconds: f32) {
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
    /// All resident geometry can cast, even when its tile is outside the camera zone.
    pub fn shadow_placed(&self) -> impl Iterator<Item = &Placed> {
        let tiles = self
            .tiles
            .values()
            .filter_map(|state| match state {
                TileState::Resident { placed, .. } => Some(placed.iter()),
                _ => None,
            })
            .flatten();
        self.shared_placed.iter().chain(tiles)
    }

    /// Only the authored road view's placements, including instances hidden from the player.
    /// Their meshes/materials have exactly the owning section's resident lifetime.
    pub(super) fn reflection_placed(&self) -> impl Iterator<Item = &ReflectionPlaced> {
        self.shared.reflection_placed.iter().chain(
            self.tiles
                .values()
                .filter_map(|state| match state {
                    TileState::Resident { resources, .. } => Some(resources.reflection_placed.iter()),
                    _ => None,
                })
                .flatten(),
        )
    }

    pub fn placed(&self) -> impl Iterator<Item = &Placed> {
        let drawn = |i: &usize| self.drawn.contains(i) || self.previous.contains(i);
        let tiles = self.tiles.iter().filter(move |(i, _)| drawn(i)).flat_map(|(_, s)| match s {
            TileState::Resident { placed, .. } => placed.as_slice(),
            TileState::Requested => &[],
        });
        self.shared_placed.iter().chain(tiles)
    }

    /// Invalidates cached environment captures after streaming or a visibility-set change.
    pub fn visual_revision(&self) -> u64 {
        self.visual_revision
    }

    /// Whether the shared sets and every tile the current zone needs are resident.
    pub fn complete(&self) -> bool {
        self.shared_ready
            && self.zones.name().is_some()
            && self.arrived.iter().all(|data| !self.wanted.contains(&data.index))
            && self.wanted_resident()
    }

    /// Keep driving while distant scenery loads. Actual local collision packs must be resident;
    /// zone names need not have a corresponding tile. Old collision/props stay until the new zone loads.
    pub fn simulation_ready(&self, [x, y]: [f32; 2], margin: f32) -> bool {
        if !self.shared_ready || self.zones.name().is_none() {
            return false;
        }
        let point = super::space::to_physics(glam::Vec3::new(x, y, 0.0));
        match self.collision.resident_near(point, margin) {
            Some(true) => true,
            // Every wanted tile is in, yet a collision pack the car's surroundings refer to is not. Nothing loads
            // that pack any more, so waiting for it would freeze the car for good. Drive on without it.
            Some(false) if self.wanted_resident() => {
                static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    log::warn!(
                        "collision near ({x:.0}, {y:.0}) refers to sections that are not loaded; driving on without them"
                    );
                }
                true
            }
            Some(false) => false,
            None => self.wanted_resident(),
        }
    }

    /// The collision of the tiles that are resident now.
    pub fn collision(&self) -> &CollisionWorld {
        &self.collision
    }

    /// The props of the resident tiles.
    pub fn props(&self) -> &PropWorld {
        &self.props
    }

    /// Collision and props together, the props mutable (cars knock them over).
    pub fn world_parts(&mut self) -> (&CollisionWorld, &mut PropWorld) {
        (&self.collision, &mut self.props)
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
