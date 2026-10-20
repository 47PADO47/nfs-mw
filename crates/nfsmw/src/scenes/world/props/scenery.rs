//! Named scenery models, separate from their physics body and streamed placement.
//! Spec: `docs/specs/pursuit-breakers.md`.

pub(in crate::scenes::world) mod breaker_sites;
mod effects;
mod explosion;
#[cfg(test)]
pub(in crate::scenes::world) mod ram;
mod spawn;
#[cfg(test)]
mod tests;

use super::super::events::{Effect, drain, queue};
use super::PropWorld;
use blackbox_attrib::vlt_hash;
use blackbox_collision::BoundsSet;
use blackbox_render::{Instance, MeshHandle};
use blackbox_scenery::ModelHierarchy;
use blackbox_sequence::{Player, QueueMode};
use glam::{Mat4, Vec3};
use nfsmw_data::{
    car::physics::PhysicsData,
    world::{
        PropDynamics,
        sequences::{Sequence, TAGS, WorldEvent},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::{Rc, Weak},
    sync::Arc,
};

pub struct ModelParts {
    pub hierarchy: Arc<ModelHierarchy>,
    pub meshes: BTreeMap<usize, MeshHandle>,
    /// Art availability is separate from GPU handles for installation-only verification.
    pub available: BTreeSet<usize>,
}

pub(super) struct SceneryModel {
    parts: Option<Arc<ModelParts>>,
    bounds: Arc<BoundsSet>,
    bounds_root: usize,
    render_root: usize,
    key: u32,
    sequence: Option<Sequence>,
    player: Option<Player>,
    failed: bool,
    hidden: bool,
    hidden_parts: BTreeSet<u32>,
    spawned: BTreeSet<u32>,
    triggered: bool,
    /// A pursuit breaker zone opened from this model since it was placed (`EBreakerStopCops`).
    fired: bool,
    owner: Weak<()>,
    parent_owner: Option<Rc<()>>,
}

impl SceneryModel {
    pub fn placed(parts: Option<Arc<ModelParts>>, bounds: Arc<BoundsSet>, spec: &PropDynamics) -> Self {
        let owner = Rc::new(());
        Self {
            parts,
            bounds,
            bounds_root: 0,
            render_root: 0,
            key: spec.event_sequencer,
            sequence: None,
            player: None,
            failed: false,
            hidden: false,
            hidden_parts: BTreeSet::new(),
            spawned: BTreeSet::new(),
            triggered: false,
            fired: false,
            owner: Rc::downgrade(&owner),
            parent_owner: Some(owner),
        }
    }

    fn initialize(&mut self, data: &PhysicsData) {
        if self.player.is_some() || self.failed || self.key == 0 {
            return;
        }
        let Some(sequence) = data.sequence(self.key) else {
            self.failed = true;
            return;
        };
        match Player::new(&sequence.package, sequence.engine, TAGS) {
            Ok(player) => {
                self.player = Some(player);
                self.sequence = Some(sequence);
            }
            Err(error) => {
                self.failed = true;
                log::warn!("scenery sequence {:08X}: {error}", self.key);
            }
        }
    }

    fn reset(&mut self) {
        self.sequence = None;
        self.player = None;
        self.failed = false;
        self.hidden = false;
        self.triggered = false;
        self.fired = false;
        self.hidden_parts.clear();
        self.spawned.clear();
        let owner = Rc::new(());
        self.owner = Rc::downgrade(&owner);
        self.parent_owner = Some(owner);
    }

    fn instances(&self, transform: Mat4, out: &mut Vec<Instance>) {
        if self.hidden || self.owner.strong_count() == 0 {
            return;
        }
        let Some(parts) = &self.parts else {
            return;
        };
        let mut pending = vec![self.render_root];
        while let Some(index) = pending.pop() {
            let node = &parts.hierarchy.nodes[index];
            if self.hidden_parts.contains(&node.name_hash) {
                continue;
            }
            if let Some(&mesh) = parts.meshes.get(&index) {
                out.push(Instance::new(mesh, transform));
            }
            let start = usize::from(node.first_child);
            for child in (start..start + usize::from(node.child_count)).rev() {
                if parts.hierarchy.nodes[child].flags & 1 == 0 {
                    pending.push(child);
                }
            }
        }
    }
}

pub(super) struct Stimulus {
    id: u32,
    system: Option<u32>,
    stimulus: u32,
    mode: QueueMode,
}

#[derive(Default)]
pub(super) struct SceneryEvents {
    pub models: BTreeMap<u32, SceneryModel>,
    pending: VecDeque<Stimulus>,
    waves: Vec<explosion::Wave>,
    /// Points where a petrol-station explosion started since the world last drained them.
    blasts: Vec<Vec3>,
    stops: Vec<StopArea>,
    common_sounds: Vec<crate::audio::CommonEvent>,
    /// The last pursuit breaker events, for the console (`breaker_sites.rs`).
    history: VecDeque<String>,
    /// Scratch for one scenery tick: events waiting for dispatch, and one model's drained output.
    ready: VecDeque<(u32, WorldEvent)>,
    drained: Vec<Effect>,
    /// Scratch for one explosion sweep: the sorted body ids and the stimuli the sweep queues.
    sweep_ids: Vec<u32>,
    sweep_stimuli: Vec<(u32, u32)>,
}

#[derive(Clone, Copy, Debug)]
pub struct StopArea {
    pub position: Vec3,
    pub radius: f32,
    pub duration: f32,
}

impl PropWorld {
    pub fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        std::mem::take(&mut self.scenery.common_sounds)
    }

    pub fn model_instances(&self, id: u32, transform: Mat4, out: &mut Vec<Instance>) -> bool {
        let Some(model) = self.scenery.models.get(&id) else {
            return false;
        };
        if model.parts.is_some() || model.hidden {
            model.instances(transform, out);
            return true;
        }
        false
    }

    pub(super) fn reset_model(&mut self, id: u32) {
        if let Some(model) = self.scenery.models.get_mut(&id) {
            model.reset();
        }
        self.remove_orphaned_fragments();
    }

    fn enqueue(&mut self, id: u32, stimulus: u32) {
        self.scenery.pending.push_back(Stimulus { id, system: None, stimulus, mode: QueueMode::Allow });
    }

    pub(super) fn contact_event(&mut self, id: u32, was_attached: bool, closing: f32) {
        let Some(model) = self.scenery.models.get_mut(&id) else {
            return;
        };
        let Some(c) = self.colliders.get(&id) else {
            return;
        };
        let (Some(body), Some(spec)) = (&c.body, &c.shape.dynamics) else {
            return;
        };
        let detached = was_attached && !body.attached;
        let trigger = !model.triggered && !spec.no_trigger && closing >= 1.0;
        model.triggered |= trigger;
        if trigger {
            self.enqueue(id, vlt_hash("IMPACT_1"));
        }
        if detached {
            self.enqueue(id, vlt_hash("WORLD_DETACHED"));
        }
    }

    pub fn update_scenery(&mut self, data: &PhysicsData) -> Vec<StopArea> {
        let mut events = std::mem::take(&mut self.scenery.ready);
        let mut out = std::mem::take(&mut self.scenery.drained);
        for (&id, model) in &mut self.scenery.models {
            model.initialize(data);
            let (Some(sequence), Some(player)) = (&model.sequence, &mut model.player) else {
                continue;
            };
            if model.failed {
                continue;
            }
            out.clear();
            let result = player
                .advance(&sequence.package, f64::from(super::super::drive::STEP))
                .map_err(anyhow::Error::from)
                .and_then(|_| drain(sequence, player, None, &mut out));
            if let Err(error) = result {
                model.failed = true;
                log::warn!("scenery {id}: {error:#}");
            }
            events.extend(out.drain(..).map(|e| (id, e.event)));
        }
        let mut remaining = 4096;
        while !events.is_empty() || !self.scenery.pending.is_empty() {
            if remaining == 0 {
                self.scenery.pending.clear();
                log::warn!("scenery dispatch exceeded 4096 events in one tick");
                break;
            }
            remaining -= 1;
            if let Some((id, event)) = events.pop_front() {
                self.dispatch_scenery(id, event, data);
                continue;
            }
            let Some(stimulus) = self.scenery.pending.pop_front() else {
                continue;
            };
            let Some(model) = self.scenery.models.get_mut(&stimulus.id) else {
                continue;
            };
            model.initialize(data);
            if model.failed {
                continue;
            }
            let (Some(sequence), Some(player)) = (&model.sequence, &mut model.player) else {
                continue;
            };
            out.clear();
            let result = player
                .stimulate(&sequence.package, stimulus.system, stimulus.stimulus, stimulus.mode, None)
                .map_err(anyhow::Error::from)
                .and_then(|_| drain(sequence, player, None, &mut out));
            if let Err(error) = result {
                model.failed = true;
                log::warn!("scenery {}: {error:#}", stimulus.id);
            }
            events.extend(out.drain(..).map(|e| (stimulus.id, e.event)));
        }
        // A dispatch cap can leave events behind; they are dropped, as before, and the buffers are kept.
        events.clear();
        out.clear();
        self.scenery.ready = events;
        self.scenery.drained = out;
        std::mem::take(&mut self.scenery.stops)
    }
}
