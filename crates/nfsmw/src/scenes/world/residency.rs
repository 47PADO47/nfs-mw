//! Section residency: shared sets once, map tiles by zone (`docs/specs/visible-sections.md`).

use std::collections::{HashMap, HashSet, VecDeque};

use blackbox_collision::{CollisionWorld, Grid};
use blackbox_gfx::RenderBackend;
use blackbox_streaming::{StreamingSection, VisibleSections};
use nfsmw_data::world::{GlobalTextures, PropCatalog, SectionData, Streamer, StreamerEvent};

use super::keys;
use super::props::PropWorld;
use super::resident::{Placed, ReflectionPlaced, SectionResources, resolve_placed, resolve_reflections, take_prepared};
use super::zone::Zones;
mod cull;
mod loading;
mod parts;
mod ready;
mod shadow_support;
mod shared_upload;
pub(super) mod start;
mod texture_detail;
mod traffic_support;
mod upload;
use crate::modding::content::scenery::MapPack;

#[cfg(test)]
mod installed_tests;
mod mod_tiles;
mod release;
mod scenery_groups;
#[cfg(test)]
mod shadow_tests;
#[cfg(test)]
mod tests;

/// Requests in flight at once.
const MAX_IN_FLIGHT: usize = 8;
/// Bytes of section data uploaded per frame while the world is shown (uploads run on the render thread): the
/// texture data the GPU receives, vertices and indices, see `SectionData::upload_bytes`. The first arrived section
/// of a frame always goes through, so a section bigger than this still loads, one per frame.
const UPLOAD_BYTES_PER_FRAME: usize = 4 * 1024 * 1024;
/// The same budget until the world has been ready once. The frontend keeps the world concealed until then, so a
/// frame may take longer to get it ready sooner.
const CONCEALED_UPLOAD_BYTES_PER_FRAME: usize = 32 * 1024 * 1024;

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
    texture_discard: Option<u32>,
    traffic_support: traffic_support::TrafficSupport,
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
    /// The shared sets while they go to the GPU over several frames.
    shared_upload: Option<shared_upload::SharedUpload>,
    /// Whether the residency has been complete once (`complete`): from then on the world is shown, and uploads use the
    /// smaller budget.
    settled: bool,
    tiles: HashMap<usize, TileState>,
    /// Loaded sections waiting for the render thread, oldest first.
    arrived: VecDeque<SectionData>,
    zones: Zones,
    /// Whether the camera has been in a zone yet. A zone is never left once entered, so this stays true.
    has_zone: bool,
    /// Tiles the current zone loads, drawn ones first.
    wanted: Vec<usize>,
    /// The same tiles as a set, for lookups.
    wanted_set: HashSet<usize>,
    /// Additional light support, independent of colour visibility and readiness.
    shadow_wanted: Vec<usize>,
    shadow_range: f32,
    shadow_direction: Option<[f32; 2]>,
    shadow_extent: [f32; 2],
    shadow_candidates: Vec<(usize, f32)>,
    shadow_eligible: usize,
    /// What the shadow support was last worked out for.
    shadow_key: Option<shadow_support::ShadowKey>,
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
    /// The scenery group of each override index that has one (`WorldIndex::override_groups`), so hidden grouped
    /// instances are kept for the career gates.
    override_groups: std::collections::HashMap<u16, u16>,
    /// Changes when the scenery available to a view changes, including while the clock is paused.
    visual_revision: u64,
    /// The first residency index of a mod's map pack: the install's sections come before it.
    mod_base: usize,
    /// The mods' map packs, by `index - mod_base`, and one streamer each for their files.
    mod_packs: Vec<MapPack>,
    mod_streamers: Vec<Streamer>,
    /// The map packs the camera wants (near an anchor, or pinned), as residency indices.
    mod_wanted: Vec<usize>,
    /// Map packs an interior holds loaded (`pin`).
    pinned: HashSet<usize>,
    /// Scratch for `unload_unwanted`, kept between frames so a frame allocates nothing.
    keep_scratch: HashSet<usize>,
    unwanted_scratch: Vec<usize>,
    /// Scratch for `request_wanted`.
    request_scratch: Vec<usize>,
}

impl Residency {
    /// A residency over `streamer`, which is asked for the shared sets here. The world starts its streamer earlier
    /// and uses [`Residency::adopt`]; this is for tests that build one by hand.
    #[cfg(test)]
    pub fn new(
        sections: Vec<StreamingSection>,
        visible: VisibleSections,
        streamer: Streamer,
        globals: GlobalTextures,
        collision_grid: Option<Grid>,
        catalog: PropCatalog,
        mod_packs: Vec<MapPack>,
    ) -> Self {
        start::request_shared(&streamer, &sections);
        let started = start::Started { streamer, requested: Vec::new() };
        Self::adopt(sections, visible, started, globals, collision_grid, catalog, mod_packs)
    }

    /// A residency over a streamer that is already working (`start::begin`): the shared sets and the first tiles were
    /// requested when it started, so they have been loading while the rest of the world was read.
    pub fn adopt(
        mut sections: Vec<StreamingSection>,
        visible: VisibleSections,
        started: start::Started,
        globals: GlobalTextures,
        collision_grid: Option<Grid>,
        catalog: PropCatalog,
        mod_packs: Vec<MapPack>,
    ) -> Self {
        let shared_total = sections.iter().filter(|section| !section.is_spatial()).count();
        let mod_base = sections.len();
        let zones = Zones::new(visible, &sections);
        // A mod pack is a section of its own file: its streamer reads that file from offset 0, as entry 0.
        let mod_streamers =
            mod_packs.iter().map(|pack| Streamer::spawn(pack.path.clone(), vec![pack.section.clone()], 1)).collect();
        sections.extend(mod_packs.iter().map(|pack| pack.section.clone()));
        let tiles = started.requested.iter().map(|&i| (i, TileState::Requested)).collect();
        Self {
            texture_discard: None,
            traffic_support: Default::default(),
            shared_total,
            zones,
            sections,
            streamer: started.streamer,
            shared_pending: Vec::new(),
            globals,
            shared: SectionResources::default(),
            sky: super::sky::Sky::default(),
            shared_placed: Vec::new(),
            shared_ready: false,
            shared_upload: None,
            settled: false,
            tiles,
            arrived: VecDeque::new(),
            has_zone: false,
            wanted: Vec::new(),
            wanted_set: HashSet::new(),
            shadow_wanted: Vec::new(),
            shadow_range: 0.0,
            shadow_direction: None,
            shadow_extent: [1.0, 0.0],
            shadow_candidates: Vec::new(),
            shadow_eligible: 0,
            shadow_key: None,
            drawn: HashSet::new(),
            previous: HashSet::new(),
            unresolved: 0,
            collision: CollisionWorld::new(collision_grid),
            props: PropWorld::default(),
            catalog,
            override_groups: Default::default(),
            visual_revision: 0,
            mod_base,
            mod_packs,
            mod_streamers,
            mod_wanted: Vec::new(),
            pinned: HashSet::new(),
            keep_scratch: HashSet::new(),
            unwanted_scratch: Vec::new(),
            request_scratch: Vec::new(),
        }
    }

    /// Request, upload and drop sections for a camera at map position (x, y).
    #[cfg(test)]
    pub fn update(&mut self, renderer: &mut dyn RenderBackend, x: f32, y: f32) {
        self.update_centres(renderer, &[[x, y]]);
    }

    /// Follow several streaming centres at once (a co-op player each, `docs/specs/coop-career.md` section 10): the
    /// tiles of all their zones load. The first centre is the lead's; shadows and map packs follow it.
    pub fn update_centres(&mut self, renderer: &mut dyn RenderBackend, centres: &[[f32; 2]]) {
        let Some(&[x, y]) = centres.first() else { return };
        let _trace = blackbox_trace::scope("World: streaming residency / uploads");
        let _span = bevy_log::info_span!("world::residency").entered();
        self.sync_bc_support(renderer);
        self.receive();
        if !self.shared_ready {
            self.finish_shared(renderer);
            return;
        }
        if self.zones.update_many(centres) {
            // A change only ever enters a zone: `Zones::update` keeps the current one when the camera leaves it.
            self.has_zone = true;
            self.switch_zone(renderer, centres);
        } else {
            self.update_shadow_support(renderer, [x, y]);
        }
        self.update_traffic_support();
        self.update_mod_wanted(x, y);
        self.upload_arrived(renderer);
        self.unload_unwanted(renderer);
        self.request_wanted();
        self.request_mod();
        self.settled = self.settled || self.complete();
    }

    /// The loader threads decode the textures this renderer cannot draw block-compressed. The streamers are spawned
    /// before a renderer exists, so the renderer's support is passed in here, every frame.
    fn sync_bc_support(&self, renderer: &dyn RenderBackend) {
        let supported = renderer.capabilities().compressed_bc;
        self.streamer.set_bc_supported(supported);
        for streamer in &self.mod_streamers {
            streamer.set_bc_supported(supported);
        }
    }

    fn switch_zone(&mut self, renderer: &dyn RenderBackend, centres: &[[f32; 2]]) {
        self.visual_revision = self.visual_revision.wrapping_add(1);
        self.previous = std::mem::replace(&mut self.drawn, self.zones.drawn());
        self.wanted = self.zones.wanted();
        // Nearest first within the drawn tiles, then within the rest.
        start::order_wanted(&mut self.wanted, &self.drawn, &self.sections, centres);
        self.wanted_set.clear();
        self.wanted_set.extend(self.wanted.iter().copied());
        self.shadow_key = None;
        self.traffic_support.computed = false;
        self.update_shadow_support(renderer, centres[0]);
        // Requests the new zone does not need are dropped: the workers skip them if they have not started them, and
        // what arrives anyway is ignored. Map packs are not in any zone's list: they are kept by their own wants
        // (`mod_tiles.rs`).
        let retained_requests: Vec<_> = self
            .shadow_wanted
            .iter()
            .copied()
            .filter(|i| !self.wanted_set.contains(i) && matches!(self.tiles.get(i), Some(TileState::Requested)))
            .take(shadow_support::MAX_SHADOW_IN_FLIGHT)
            .collect();
        let (streamer, wanted, mod_base) = (&self.streamer, &self.wanted_set, self.mod_base);
        self.tiles.retain(|i, s| {
            let keep = wanted.contains(i)
                || *i >= mod_base
                || retained_requests.contains(i)
                || matches!(s, TileState::Resident { .. });
            if !keep {
                streamer.cancel(*i);
            }
            keep
        });
        log::debug!("zone {:?}: {} tiles drawn, {} loaded", self.zones.name(), self.drawn.len(), self.wanted.len());
    }

    fn receive(&mut self) {
        while let Some(event) = self.streamer.poll() {
            self.handle(event);
        }
        self.receive_mod();
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
        self.keep_scratch.clear();
        self.keep_scratch.extend(self.wanted.iter().chain(&self.mod_wanted).copied());
        // Only the bounded support selection can retain extra geometry. Previous visits
        // must not accumulate every tile inside the larger hysteresis region.
        let mut unwanted = std::mem::take(&mut self.unwanted_scratch);
        unwanted.clear();
        unwanted.extend(
            self.tiles.keys().copied().filter(|i| !self.keep_scratch.contains(i) && !self.shadow_wanted.contains(i)),
        );
        for i in unwanted.drain(..) {
            if let Some(TileState::Resident { resources, packs, .. }) = self.tiles.remove(&i) {
                self.visual_revision = self.visual_revision.wrapping_add(1);
                resources.release(renderer);
                self.retain_traffic_collision(i, &packs);
                self.props.remove_tile(i);
                for section in packs {
                    self.remove_collision(section, i);
                }
            }
        }
        self.unwanted_scratch = unwanted;
    }

    fn request_wanted(&mut self) {
        let in_flight = self.tiles.values().filter(|s| matches!(s, TileState::Requested)).count();
        let free = MAX_IN_FLIGHT.saturating_sub(in_flight);
        let mut missing = std::mem::take(&mut self.request_scratch);
        missing.clear();
        missing.extend(self.wanted.iter().copied().filter(|i| !self.tiles.contains_key(i)).take(free));
        for &i in &missing {
            self.tiles.insert(i, TileState::Requested);
            self.streamer.request(i);
        }
        self.request_scratch = missing;
        self.request_shadow_support();
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
        self.shared_placed.iter().chain(tiles).filter(|p| self.group_shown(p))
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
        // A resident map pack is drawn whatever the zone: no zone's list names it.
        let base = self.mod_base;
        let drawn = move |i: &usize| self.drawn.contains(i) || self.previous.contains(i) || *i >= base;
        let tiles = self.tiles.iter().filter(move |(i, _)| drawn(i)).flat_map(|(_, s)| match s {
            TileState::Resident { placed, .. } => placed.as_slice(),
            TileState::Requested => &[],
        });
        self.shared_placed.iter().chain(tiles).filter(|p| self.group_shown(p))
    }

    /// Switches scenery group `group` in or out of the collision world. The collision world is built once and never
    /// replaced, so the group stays set across section loads and unloads.
    /// The props of the resident tiles.
    pub fn props(&self) -> &PropWorld {
        &self.props
    }

    /// Collision and props together, the props mutable (cars knock them over).
    pub fn world_parts(&mut self) -> (&CollisionWorld, &mut PropWorld) {
        (&self.collision, &mut self.props)
    }
}
