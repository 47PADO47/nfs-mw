//! Section override persistence; distinct from simulation cleanup. Spec: `prop-lifetime.md`.

use std::collections::{BTreeMap, HashMap};

use super::PropWorld;

const RESTORE_DELAY: f64 = 180.0;

#[derive(Default)]
pub(super) struct Streaming {
    now: f64,
    reset_locked: bool,
    sections: HashMap<usize, Section>,
    restored: Vec<usize>,
}

struct Section {
    unloaded_at: f64,
    /// Only IDs and consumed flags survive; assets and simulation state belong to residency.
    hidden: BTreeMap<u32, bool>,
}

impl Streaming {
    pub(super) fn advance(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.now += f64::from(dt);
        }
    }

    pub(super) fn reload(&mut self, tile: usize) -> BTreeMap<u32, bool> {
        let Some(section) = self.sections.remove(&tile) else { return BTreeMap::new() };
        if !self.reset_locked && section.unloaded_at + RESTORE_DELAY < self.now {
            self.restored.push(tile);
            return BTreeMap::new();
        }
        if section.hidden.is_empty() {
            self.restored.push(tile);
        }
        section.hidden
    }
}

impl PropWorld {
    /// Resident placement ownership for section-scoped consumers such as map icons.
    pub fn section_for(&self, id: u32) -> Option<usize> {
        self.tiles.iter().find_map(|(&tile, ids)| ids.contains(&id).then_some(tile))
    }

    /// The original race-status gate suppresses expiry checks, while simulated time keeps advancing.
    pub fn set_streaming_reset_locked(&mut self, locked: bool) {
        self.streaming.reset_locked = locked;
    }

    /// Sections whose consumed state expired or which had no hidden overrides, for map-icon restoration.
    pub fn take_restored_sections(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.streaming.restored)
    }

    pub(super) fn remember_streamed(&mut self, tile: usize, ids: &[u32]) {
        let hidden = ids
            .iter()
            .filter_map(|&id| {
                let collider = self.colliders.get(&id)?;
                let moved = collider.body.as_ref().is_some_and(|body| body.moved);
                let fired = self.scenery.models.get(&id)?.streaming_state(moved)?;
                Some((id, fired))
            })
            .collect();
        self.streaming.sections.insert(tile, Section { unloaded_at: self.streaming.now, hidden });
    }

    pub(super) fn restore_streamed_hidden(&mut self, id: u32, fired: bool) {
        if let Some(collider) = self.colliders.get_mut(&id) {
            collider.collidable = false;
        }
        if let Some(model) = self.scenery.models.get_mut(&id) {
            model.restore_streamed_hidden(fired);
        }
    }
}

#[cfg(test)]
mod tests;
