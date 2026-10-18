//! The scenery groups the world has switched on. The career drives this set when a region gate opens or closes
//! (`docs/specs/career-regions.md` sections 3.1 and 4). A group is the set of scenery and collision instances that
//! share one group number: group 0 is always present, any other group only while it is in the set.
//!
//! The set is mirrored into the collision world (`set_scenery_group`): an enabled group collides, and the residency
//! lists its placed instances, so it is drawn and casts shadows.

use std::collections::BTreeSet;

use super::WorldScene;

#[cfg(test)]
#[path = "scenery_groups_tests.rs"]
mod tests;

/// The enabled scenery group numbers. Empty outside career mode.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SceneryGroups {
    enabled: BTreeSet<u16>,
}

impl SceneryGroups {
    /// Switches `group` on or off. Returns whether the set changed.
    pub fn set(&mut self, group: u16, enabled: bool) -> bool {
        if enabled {
            return self.enabled.insert(group);
        }
        self.enabled.remove(&group)
    }

    /// Whether instances of `group` are drawn and collide: group 0 always, any other group only while it is enabled.
    pub fn admits(&self, group: u16) -> bool {
        group == 0 || self.enabled.contains(&group)
    }

    /// Whether no group is switched on.
    #[allow(dead_code)] // no caller yet; kept with `admits`
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }
}

impl WorldScene {
    /// Switches scenery group `group` on or off, by its group number (the number the collision instances store).
    /// The collision world takes the group, so a closed gate blocks; hash-keyed entries would need the group table,
    /// which is not read yet.
    pub(super) fn set_scenery_group(&mut self, group: u16, enabled: bool) {
        self.scenery_groups.set(group, enabled);
        self.residency.set_collision_group(group, enabled);
    }
}
