//! Putting the shared sets (V/X/Y/Z: 2,734 models, 2,111 textures) on the GPU over several frames instead of in one.
//! The world is concealed until it is ready, so the load can take as many frames as its byte budget needs, and the
//! frontend's loading screen keeps moving meanwhile.
//!
//! The steps run in the order the one-frame version ran them, because the models of one set refer to the textures of
//! another: every set's textures, the global textures, every set's models, the global models and the sky, then the
//! placements.

use nfsmw_data::world::texture_upload_bytes;

use super::*;

/// What a placement step is charged against the budget: it uploads nothing, but it is work for the frame thread.
const PLACE_COST: usize = 1 << 20;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    /// The textures of `pending[next]`.
    Textures,
    /// The global textures, and the animations of the global packs and of the sets.
    Globals,
    /// The models of `pending[next]`.
    Meshes,
    /// The global models, and the sky.
    GlobalSolids,
    /// The placements of `pending[next]`.
    Place,
    Done,
}

/// The shared sets part way to the GPU.
pub(super) struct SharedUpload {
    shared: SectionResources,
    pending: Vec<SectionData>,
    globals: GlobalTextures,
    stage: Stage,
    /// The set the current stage works on, for the stages that go set by set.
    next: usize,
    sky: Option<super::super::sky::Sky>,
    placed: Vec<Placed>,
    unresolved: usize,
}

/// A finished shared upload.
pub(super) struct SharedUploaded {
    pub shared: SectionResources,
    pub sky: super::super::sky::Sky,
    pub placed: Vec<Placed>,
    pub unresolved: usize,
}

impl SharedUpload {
    pub fn new(pending: Vec<SectionData>, globals: GlobalTextures, texture_discard: u32) -> Self {
        Self {
            shared: SectionResources::with_texture_detail(texture_discard),
            pending,
            globals,
            stage: Stage::Textures,
            next: 0,
            sky: None,
            placed: Vec::new(),
            unresolved: 0,
        }
    }

    /// Runs steps until the budget is spent; true once everything is uploaded. The first step of a call always runs,
    /// so a step bigger than the budget still completes, one per call.
    pub fn advance(
        &mut self,
        renderer: &mut dyn RenderBackend,
        budget: usize,
        catalog: &PropCatalog,
        groups: &std::collections::HashMap<u16, u16>,
    ) -> bool {
        let mut spent = 0;
        while self.stage != Stage::Done {
            let cost = self.step_cost();
            if spent > 0 && spent + cost > budget {
                return false;
            }
            spent += cost;
            self.step(renderer, catalog, groups);
        }
        true
    }

    /// The bytes the next step uploads. A stage with no set left costs nothing: its step only moves on.
    fn step_cost(&self) -> usize {
        match self.stage {
            Stage::Textures => self.pending.get(self.next).map_or(0, SectionData::texture_bytes),
            Stage::Globals => texture_upload_bytes(&self.globals.textures, &[]),
            Stage::Meshes => self.pending.get(self.next).map_or(0, SectionData::mesh_bytes),
            Stage::GlobalSolids => {
                let vertex = std::mem::size_of::<blackbox_gfx::Vertex>();
                self.globals.solids.iter().map(|s| s.vertices.len() * vertex + s.indices.len() * 2).sum()
            }
            Stage::Place => self.pending.get(self.next).map_or(0, |_| PLACE_COST),
            Stage::Done => 0,
        }
    }

    fn step(
        &mut self,
        renderer: &mut dyn RenderBackend,
        catalog: &PropCatalog,
        groups: &std::collections::HashMap<u16, u16>,
    ) {
        let empty = SectionResources::default();
        match self.stage {
            Stage::Textures => {
                if let Some(data) = self.pending.get(self.next) {
                    self.shared.upload_textures(renderer, data);
                }
                self.next_set(Stage::Globals);
            }
            Stage::Globals => {
                self.shared.upload_texture_list(renderer, &self.globals.textures, &[]);
                let anims = std::mem::take(&mut self.globals.anims);
                self.shared.anims.extend(anims);
                self.shared.anims.extend(self.pending.iter().flat_map(|d| d.anims.iter().cloned()));
                self.stage = Stage::Meshes;
                self.next = 0;
            }
            Stage::Meshes => {
                if let Some(data) = self.pending.get(self.next) {
                    self.shared.upload_meshes(renderer, data, &empty);
                }
                self.next_set(Stage::GlobalSolids);
            }
            Stage::GlobalSolids => {
                // The shared models of InGameA.bun (sky domes and the other SKY_* models); a stream solid of the
                // same name wins.
                self.shared.upload_solid_list(renderer, &self.globals.solids, &empty, true);
                let solids = self.pending.iter().flat_map(|data| &data.solids).chain(&self.globals.solids);
                self.sky = Some(super::super::sky::Sky::upload(renderer, solids, &self.shared));
                self.stage = Stage::Place;
                self.next = 0;
            }
            Stage::Place => {
                if let Some(data) = self.pending.get_mut(self.next) {
                    let prepared = take_prepared(data, catalog, groups);
                    let (placed, unresolved) = resolve_placed(&prepared.placed, &self.shared, &empty);
                    let mut reflected = resolve_reflections(&prepared.reflections, &self.shared, &empty);
                    for reflected in &mut reflected {
                        reflected.player_index = reflected.player_index.map(|i| i + self.placed.len());
                    }
                    self.shared.reflection_placed.extend(reflected);
                    self.placed.extend(placed);
                    self.unresolved += unresolved;
                }
                self.next_set(Stage::Done);
            }
            Stage::Done => {}
        }
    }

    /// Moves to the next set of a stage that goes set by set, and to `after` when the sets are done.
    fn next_set(&mut self, after: Stage) {
        self.next += 1;
        if self.next < self.pending.len() {
            return;
        }
        self.stage = after;
        self.next = 0;
    }

    /// The finished upload. Only valid once [`SharedUpload::advance`] returned true.
    pub fn finish(self) -> SharedUploaded {
        SharedUploaded {
            shared: self.shared,
            sky: self.sky.unwrap_or_default(),
            placed: self.placed,
            unresolved: self.unresolved,
        }
    }

    /// Frees what is on the GPU so far, for a scene that leaves the window part way.
    pub fn release(self, renderer: &mut dyn RenderBackend) {
        self.shared.release(renderer);
        if let Some(mut sky) = self.sky {
            sky.release(renderer);
        }
    }
}

impl Residency {
    /// Takes the shared sets that have arrived to the GPU, as far as this frame's byte budget goes. Once all of them
    /// are up their placements are registered and the residency is ready for tiles.
    pub(super) fn finish_shared(&mut self, renderer: &mut dyn RenderBackend) {
        if self.shared_upload.is_none() {
            if self.shared_pending.len() < self.shared_total {
                return;
            }
            let pending = std::mem::take(&mut self.shared_pending);
            let globals = std::mem::take(&mut self.globals);
            self.shared_upload = Some(SharedUpload::new(pending, globals, self.texture_discard.unwrap_or(0)));
        }
        let Some(mut upload) = self.shared_upload.take() else { return };
        let budget = self.upload_budget();
        if !upload.advance(renderer, budget, &self.catalog, &self.override_groups) {
            self.shared_upload = Some(upload);
            return;
        }
        self.install_shared(renderer, upload.finish());
    }

    fn install_shared(&mut self, renderer: &mut dyn RenderBackend, uploaded: SharedUploaded) {
        let SharedUploaded { mut shared, sky, placed, unresolved } = uploaded;
        self.sky.release(renderer);
        self.sky = sky;
        self.shared_placed = placed;
        self.unresolved += unresolved;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_set() -> SectionData {
        SectionData {
            index: 0,
            solids: Vec::new(),
            textures: Vec::new(),
            decoded: Vec::new(),
            anims: Vec::new(),
            scenery: Vec::new(),
            spawners: Vec::new(),
            collision: Vec::new(),
            prepared: None,
        }
    }

    fn catalog() -> PropCatalog {
        PropCatalog::new(Vec::new(), &blackbox_attrib::Database::new())
    }

    #[test]
    fn a_small_budget_spreads_the_sets_over_several_calls() {
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        let catalog = catalog();
        let groups = Default::default();
        let mut upload = SharedUpload::new(vec![empty_set(), empty_set(), empty_set()], GlobalTextures::default(), 0);
        // The sets cost nothing, so only the placements are charged: one per call at this budget.
        let mut calls = 0;
        while !upload.advance(&mut renderer, PLACE_COST, &catalog, &groups) {
            calls += 1;
            assert!(calls < 10, "the upload must finish");
        }
        assert!(calls >= 2, "three placement steps cannot fit one {PLACE_COST}-byte call");
        assert_eq!(upload.stage, Stage::Done);
    }

    #[test]
    fn a_big_budget_finishes_in_one_call() {
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        let catalog = catalog();
        let groups = Default::default();
        let mut upload = SharedUpload::new(vec![empty_set(), empty_set()], GlobalTextures::default(), 0);
        assert!(upload.advance(&mut renderer, usize::MAX, &catalog, &groups));
        let uploaded = upload.finish();
        assert!(uploaded.placed.is_empty());
        assert_eq!(uploaded.unresolved, 0);
    }

    #[test]
    fn no_shared_sets_is_a_finished_upload() {
        let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
        let mut upload = SharedUpload::new(Vec::new(), GlobalTextures::default(), 0);
        assert!(upload.advance(&mut renderer, 0, &catalog(), &Default::default()));
    }
}
