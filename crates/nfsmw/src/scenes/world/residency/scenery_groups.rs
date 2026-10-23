//! The scenery groups of the residency: which override indexes belong to a group, and which groups are switched on
//! (the career gates, `docs/specs/career-regions.md` section 4).

use std::collections::HashMap;

use super::Residency;
use crate::scenes::world::resident::Placed;

impl Residency {
    /// The scenery group table of the track (set once at open, before sections arrive).
    pub(in crate::scenes::world) fn set_override_groups(&mut self, groups: HashMap<u16, u16>) {
        self.override_groups = groups;
    }

    /// Switches a scenery group on or off in the collision world, which the placed lists also follow.
    pub(in crate::scenes::world) fn set_collision_group(&mut self, group: u16, enabled: bool) {
        self.collision.set_group_enabled(group, enabled);
    }

    /// Makes scenery group `group`'s barriers walls from both sides for the cars, or from their front only again.
    pub(in crate::scenes::world) fn set_collision_two_sided(&mut self, group: u16, two_sided: bool) {
        self.collision.set_group_two_sided(group, two_sided);
    }

    /// Whether `p` is in the world: an ungrouped instance always, a grouped one (a career gate) while its scenery
    /// group is switched on (`set_collision_group`), so drawing, shadows and collision agree.
    pub(super) fn group_shown(&self, p: &Placed) -> bool {
        p.group == 0 || self.collision.group_enabled(p.group)
    }
}
