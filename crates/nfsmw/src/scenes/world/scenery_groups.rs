//! The scenery groups the world has switched on. The career drives this set when a region gate opens or closes
//! (`docs/specs/career-regions.md` sections 3.1 and 4), and a race switches its walls on for its run
//! (`docs/specs/race-barriers.md`). A group is the set of scenery and collision instances that share one group
//! number: group 0 is always present, any other group only while it is in the set.
//!
//! The set is mirrored into the collision world (`switch_scenery_group`): an enabled group collides, and the residency
//! lists its placed instances, so it is drawn. A group switched on with the star is *turned*: its turnable pieces are
//! drawn mirrored along their local x axis (`visibility.rs`).

use std::collections::BTreeMap;

use blackbox_roads::{ActiveBarrier, TrackBarrier};
use blackbox_scenery::{SceneryGroup, find_group};

use super::WorldScene;

#[cfg(test)]
#[path = "scenery_groups_tests.rs"]
mod tests;

/// The group the engine switches on after every clear: the junk-pile door of the final race
/// (`docs/specs/career-regions.md` section 3.2).
pub(super) const DOOR_GROUP: &str = "SCENERY_GROUP_DOOR";

/// The enabled scenery group numbers, each with its turned bit, and the track's group table (names to numbers).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SceneryGroups {
    /// Group number to "turned" (switched on with the star).
    enabled: BTreeMap<u16, bool>,
    /// The track's groups (`0x34109`), for the lookup by name hash.
    table: Vec<SceneryGroup>,
}

impl SceneryGroups {
    /// An empty set over the track's group table.
    pub fn new(table: Vec<SceneryGroup>) -> Self {
        Self { enabled: BTreeMap::new(), table }
    }

    /// Switches `group` on (not turned) or off; a group already on keeps its turn. Returns whether the set changed.
    pub fn set(&mut self, group: u16, enabled: bool) -> bool {
        if !enabled {
            return self.enabled.remove(&group).is_some();
        }
        if self.enabled.contains_key(&group) {
            return false;
        }
        self.enabled.insert(group, false);
        true
    }

    /// Puts `group` in `state`: `Some(turned)` switches it on with that turn, `None` off.
    pub fn switch(&mut self, group: u16, state: Option<bool>) {
        match state {
            Some(turned) => self.enabled.insert(group, turned),
            None => self.enabled.remove(&group),
        };
    }

    /// `Some(turned)` while `group` is on, `None` while it is off.
    pub fn state(&self, group: u16) -> Option<bool> {
        self.enabled.get(&group).copied()
    }

    /// Whether instances of `group` are drawn and collide: group 0 always, any other group only while it is enabled.
    pub fn admits(&self, group: u16) -> bool {
        group == 0 || self.enabled.contains_key(&group)
    }

    /// Whether `group` is on and turned: its turnable pieces are drawn mirrored.
    pub fn turned(&self, group: u16) -> bool {
        self.state(group) == Some(true)
    }

    /// Whether `group`'s barriers block from both sides while it is on: every group but a drive-through one, so a
    /// race's walls and a closed gate stop a car whichever way it meets them [ours].
    pub fn two_sided(&self, group: u16) -> bool {
        group != 0 && !self.table.iter().any(|g| g.number == group && g.drive_through)
    }

    /// Whether no group is switched on.
    #[allow(dead_code)] // no caller yet; kept with `admits`
    pub fn is_empty(&self) -> bool {
        self.enabled.is_empty()
    }

    /// The group whose name is `name` (case sensitive, by `bStringHash`), if the track has one.
    pub fn named(&self, name: &str) -> Option<&SceneryGroup> {
        find_group(&self.table, blackbox_hash::bstring_hash(name))
    }

    /// The barrier lines that are on: those whose group (by name hash) is switched on, each with its group's
    /// drive-through byte.
    pub fn active_barriers(&self, lines: &[TrackBarrier]) -> Vec<ActiveBarrier> {
        lines
            .iter()
            .filter_map(|line| {
                let group = find_group(&self.table, line.group_hash)?;
                let on = group.number != 0 && self.enabled.contains_key(&group.number);
                on.then_some(ActiveBarrier { a: line.a, b: line.b, drive_through: group.drive_through })
            })
            .collect()
    }
}

impl WorldScene {
    /// Switches scenery group `group` on or off, by its group number (the number the collision instances store).
    /// The collision world takes the group, so a closed gate blocks.
    pub(super) fn set_scenery_group(&mut self, group: u16, enabled: bool) {
        self.scenery_groups.set(group, enabled);
        self.residency.set_collision_group(group, enabled);
        self.residency.set_collision_two_sided(group, enabled && self.scenery_groups.two_sided(group));
    }

    /// Puts scenery group `group` in `state` (`Some(turned)` on, `None` off), in the drawn set and the collision world.
    pub(super) fn switch_scenery_group(&mut self, group: u16, state: Option<bool>) {
        self.scenery_groups.switch(group, state);
        self.residency.set_collision_group(group, state.is_some());
        self.residency.set_collision_two_sided(group, state.is_some() && self.scenery_groups.two_sided(group));
    }

    /// The groups the engine has on after a clear: the door group, not turned. Called once the world is built.
    pub(super) fn switch_default_scenery_groups(&mut self) {
        let Some(door) = self.scenery_groups.named(DOOR_GROUP).map(|g| g.number) else { return };
        self.switch_scenery_group(door, Some(false));
        self.refresh_road_barriers();
    }

    /// Flags the road segments that cross a barrier line of a switched-on group (the region gates, the door), so the
    /// GPS routes around closed roads. Called after each batch of switches, as the engine recomputes after each enable
    /// pass. While a race runs the flags stay clear (`race/barriers.rs` says why).
    pub(super) fn refresh_road_barriers(&mut self) {
        let racing = self.race.is_some();
        let Some(traffic) = self.traffic.as_mut() else { return };
        let active = match racing {
            true => Vec::new(),
            false => self.scenery_groups.active_barriers(traffic.track_barriers()),
        };
        let marked = traffic.mark_road_barriers(&active);
        log::debug!("road barriers: {} line(s) on, {marked} segment(s) closed to routes", active.len());
    }
}
