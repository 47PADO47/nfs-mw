//! Main-view policy and bounded history, separate from shadow/reflection selection.
use super::resident::Placed;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod tests;
use crate::settings::{DetailSettings, detail::DetailRange};
use blackbox_gfx::InstanceKey;
use blackbox_scenery::{LodObject, LodPolicy, LodView, layout::LodRules};
use std::collections::HashMap;

/// Keys are the renderer's instance names; a fixed fast hasher is plenty for them and stays out of the frame's way.
type Selections = HashMap<InstanceKey, (usize, u32), foldhash::fast::FixedState>;

#[derive(Default)]
pub(super) struct DetailState {
    pub settings: DetailSettings,
    /// The rear-view mirror's far limit in metres (`ReflectionDistance::mirror_far`), or `None` for no limit. The
    /// scene refreshes it each frame so the mirror's scan can apply the limit itself.
    pub mirror_far: Option<f32>,
    /// The slot each remembered instance showed last frame, with the pass (`epoch`) that last chose it.
    selections: Selections,
    epoch: u32,
}
impl DetailState {
    pub fn configure(&mut self, settings: DetailSettings) {
        if self.settings != settings {
            self.selections.clear();
        }
        self.settings = settings;
    }
    pub fn begin(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }
    /// Forgets the instances this pass did not choose a slot for.
    pub fn finish(&mut self) {
        let epoch = self.epoch;
        self.selections.retain(|_, (_, chosen)| *chosen == epoch);
    }
    pub fn choose(&mut self, p: &Placed, position: [f32; 3], view: &LodView, rules: &LodRules) -> Option<usize> {
        if !self.settings.world.admits(p.flags) {
            return None;
        }
        let extended = self.settings.geometry_range != DetailRange::Original;
        let previous = match extended {
            true => self.selections.get(&p.key).map(|&(slot, _)| slot),
            false => None,
        };
        let policy = LodPolicy {
            coarse_only: self.settings.world.coarse(),
            highest: self.settings.highest_world,
            geometry_scale: self.settings.geometry_range.factor(),
            visibility_scale: self.settings.visibility_range.factor(),
            hysteresis: match extended {
                true => 0.1,
                false => 0.0,
            },
            previous,
        };
        let slot = rules.choose_with(
            view,
            LodObject {
                position,
                radius: p.radius,
                flags: p.flags,
                slots: p.lods.map(|m| m.is_some()),
                detailed: p.detailed,
            },
            policy,
        )?;
        if extended {
            self.selections.insert(p.key, (slot, self.epoch));
        }
        Some(slot)
    }
}
