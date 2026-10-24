//! Roadblocks in the world: a block of cop cars, sawhorses and spike strips across the road 250 m ahead of the player,
//! laid out by `blackbox_pursuit::roadblock`. The cars stop where they are placed, with the brakes held
//! (`AIGoalStaticRoadBlock`); the sawhorses are bodies in the prop world and the strips are tested by the tire box.
//!
//! - [`site`]: where (the original's checks), [`build`]: the layout, the cars and the props (the rewrite's occupancy and
//!   on-road checks), [`director`]: when (the request lottery, the heavy support request, the dodge, the cheat rule, the
//!   release), [`state`]: the standing roadblocks, [`props`]: the installed models and bodies.
//!
//! Specs: `docs/specs/ai-pursuit-roadblocks.md`, `docs/specs/ai-pursuit-support.md`.

use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use nfsmw_data::car::physics::SurfaceTable;

use super::{PlayerView, TrafficWorld};
use crate::scenes::world::drive::CarRig;
use crate::scenes::world::props::PropWorld;

mod build;
mod director;
#[cfg(test)]
mod installed_tests;
pub(super) mod props;
mod site;
mod state;
mod steal;

pub use build::Cause;
pub use state::Roadblocks;

impl TrafficWorld {
    /// Lays a roadblock 250 m ahead of the player with no prop world (the sawhorses are left out). Returns the number
    /// of cop cars placed. The headless tests use it.
    #[cfg(test)]
    pub(in crate::scenes::world) fn place_roadblock(
        &mut self,
        view: PlayerView,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> Result<usize, String> {
        self.build_roadblock(Cause::Console, view, collision, None, surfaces)
    }

    /// As [`place_roadblock`](Self::place_roadblock), with the prop world: the `roadblock` console command.
    pub(in crate::scenes::world) fn place_roadblock_with_props(
        &mut self,
        view: PlayerView,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) -> Result<usize, String> {
        self.build_roadblock(Cause::Console, view, collision, Some(props), surfaces)
    }

    /// The props' bodies and sizes, read from the install's tables once the prop catalog is built.
    pub fn set_roadblock_physics(&mut self, catalog: &nfsmw_data::world::PropCatalog) {
        self.roadblocks.physics = Some(props::Physics::from_catalog(catalog));
    }

    /// Whether the models of the props are wanted now: a chase runs and they are not loaded or asked for yet.
    pub(in crate::scenes::world) fn wants_roadblock_models(&self) -> bool {
        self.is_pursuing() && self.roadblocks.wants_models()
    }

    /// The models of the props were asked for.
    pub(in crate::scenes::world) fn roadblock_models_asked(&mut self) {
        self.roadblocks.asked = true;
    }

    /// The models of the props did not load: the roadblocks go without them.
    pub(in crate::scenes::world) fn roadblock_models_failed(&mut self) {
        self.roadblocks.failed = true;
    }

    /// The models of the props are uploaded.
    pub(in crate::scenes::world) fn set_roadblock_models(&mut self, spike: Rc<CarRig>, sawhorse: Rc<CarRig>) {
        self.roadblocks.kit = Some(props::Kit::new(spike, sawhorse));
    }

    /// The uploaded prop rigs, taken for the teardown to free; the roadblocks go without models after it.
    pub(super) fn take_roadblock_rigs(&mut self) -> Vec<Rc<CarRig>> {
        self.roadblocks.kit.take().map(|kit| kit.into_rigs().into()).unwrap_or_default()
    }

    /// The parts a sawhorse's sequence releases when it breaks (from the shared scenery).
    pub(in crate::scenes::world) fn set_sawhorse_parts(
        &mut self,
        parts: Option<std::sync::Arc<crate::scenes::world::props::scenery::ModelParts>>,
    ) {
        if let Some(kit) = self.roadblocks.kit.as_mut() {
            kit.set_sawhorse_parts(parts);
        }
    }

    /// The strips of the roadblocks drawn from their model; the strips of a world without the model are drawn by
    /// `spike_geometry`.
    pub fn roadblock_instances(&self, out: &mut Vec<blackbox_render::Instance>) {
        let Some(kit) = self.roadblocks.kit.as_ref() else { return };
        for frame in self.roadblocks.list.iter().flat_map(|block| block.strip_frames.iter()) {
            kit.draw_strip(*frame, out);
        }
    }

    /// Every standing roadblock's spike strips as (roadblock id, strip index, frame in render space), in roadblock
    /// order: what a server replicates of them (`net_export.rs`).
    pub fn spike_strips(&self) -> impl Iterator<Item = (u32, usize, glam::Mat4)> + '_ {
        self.roadblocks
            .list
            .iter()
            .flat_map(|block| block.strip_frames.iter().enumerate().map(|(index, frame)| (block.id, index, *frame)))
    }
}
