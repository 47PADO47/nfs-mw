//! Main-view policy and bounded history, separate from shadow/reflection selection.
use super::resident::Placed;
#[cfg(test)]
mod tests;
use crate::settings::{DetailSettings, detail::DetailRange};
use blackbox_gfx::InstanceKey;
use blackbox_scenery::{LodObject, LodPolicy, LodView, layout::LodRules};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct DetailState {
    pub settings: DetailSettings,
    selections: HashMap<InstanceKey, usize>,
    seen: HashSet<InstanceKey>,
}
impl DetailState {
    pub fn configure(&mut self, settings: DetailSettings) {
        if self.settings != settings {
            self.selections.clear();
        }
        self.settings = settings;
    }
    pub fn begin(&mut self) {
        self.seen.clear();
    }
    pub fn finish(&mut self) {
        self.selections.retain(|key, _| self.seen.contains(key));
    }
    pub fn choose(&mut self, p: &Placed, position: [f32; 3], view: &LodView, rules: &LodRules) -> Option<usize> {
        if !self.settings.world.admits(p.flags) {
            return None;
        }
        let extended = self.settings.geometry_range != DetailRange::Original;
        let policy = LodPolicy {
            coarse_only: self.settings.world.coarse(),
            highest: self.settings.highest_world,
            geometry_scale: self.settings.geometry_range.factor(),
            visibility_scale: self.settings.visibility_range.factor(),
            hysteresis: match extended {
                true => 0.1,
                false => 0.0,
            },
            previous: self.selections.get(&p.key).copied(),
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
            self.selections.insert(p.key, slot);
            self.seen.insert(p.key);
        }
        Some(slot)
    }
}
