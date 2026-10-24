//! Authored scenery marker flares; see `docs/specs/environment-prop-lights.md`.

mod draw;
#[cfg(test)]
mod gpu_tests;
#[cfg(test)]
mod installed_tests;
#[cfg(test)]
mod scene_tests;
#[cfg(test)]
mod tests;
mod world;

use std::{collections::HashMap, sync::Arc};

use blackbox_gfx::{Instance, MeshHandle};
use nfsmw_data::{
    car::lights::FlareProfile,
    world::lights::{Kind, Marker},
};

use super::scenery_groups::SceneryGroups;

type FastMap<K, V> = HashMap<K, V, foldhash::fast::FixedState>;

/// Three world clocks, independent of the player and of texture animation speed.
#[derive(Default)]
pub(super) struct BlinkClock {
    elapsed: [f32; 3],
    on: [bool; 3],
}

impl BlinkClock {
    pub fn advance(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        for (i, period) in [0.5, 0.4, 0.45].into_iter().enumerate() {
            self.elapsed[i] += dt;
            if self.elapsed[i] > period {
                self.elapsed[i] = 0.0;
                self.on[i] = !self.on[i];
            }
        }
    }

    fn lit(&self, kind: Kind, world_x: f32) -> bool {
        match kind {
            Kind::AmberBlink => self.on[((world_x as i32) % 3).unsigned_abs() as usize],
            Kind::RedBlink => self.on[0],
            Kind::GreenBlink => self.on[2],
            _ => true,
        }
    }
}

pub(super) struct EnvironmentLights {
    pub blink: BlinkClock,
    profiles: Vec<(Kind, FlareProfile)>,
    sources: FastMap<MeshHandle, Arc<[Marker]>>,
    /// The placed objects that carry authored markers, built with `sources` and valid for the same scenery state.
    candidates: Vec<super::resident::Placed>,
    revision: Option<u64>,
    /// The scenery groups `candidates` was built for: switching one does not bump the revision, but it changes what
    /// the residency lists as placed.
    groups: SceneryGroups,
    detail: super::detail::DetailState,
    visible: Vec<Instance>,
}

impl EnvironmentLights {
    pub fn new(db: &blackbox_attrib::Database) -> Self {
        Self {
            blink: Default::default(),
            profiles: nfsmw_data::world::lights::profiles(db),
            sources: Default::default(),
            candidates: Vec::new(),
            revision: None,
            groups: Default::default(),
            detail: Default::default(),
            visible: Vec::new(),
        }
    }

    /// Rebuilds the marker sources and the marked placements when the resident scenery or the switched-on groups
    /// changed since the last build.
    fn refresh(&mut self, residency: &super::residency::Residency, groups: &SceneryGroups) {
        let revision = residency.visual_revision();
        if self.revision == Some(revision) && self.groups == *groups {
            return;
        }
        self.sources = residency.light_sources().collect();
        self.candidates = residency.placed().filter(|p| self.contains(p)).cloned().collect();
        self.revision = Some(revision);
        self.groups = groups.clone();
    }

    fn contains(&self, p: &super::resident::Placed) -> bool {
        p.lods.iter().flatten().any(|mesh| self.sources.contains_key(mesh))
            || p.parts.as_ref().is_some_and(|parts| parts.meshes.values().any(|mesh| self.sources.contains_key(mesh)))
    }
}
