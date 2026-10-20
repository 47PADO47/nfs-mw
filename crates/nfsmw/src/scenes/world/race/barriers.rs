//! The walls of a race: the scenery groups its `Barriers` list names are switched on when the race starts and put
//! back when it ends, finishes, is stopped or quit (`docs/specs/race-barriers.md`). Each entry is read the way the
//! engine reads it (a leading `*` turns the pieces, names outside the accepted prefixes are skipped), the name's hash
//! finds the group, and the group's pieces are drawn and collide while the race runs.

use nfsmw_career::regions::parse_barrier_entry;

use crate::scenes::world::WorldScene;
use crate::scenes::world::scenery_groups::SceneryGroups;

/// One group a list switches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Switch {
    pub group: u16,
    /// The entry was starred: the group's turnable pieces are mirrored.
    pub turned: bool,
}

/// The groups `list` switches on, in list order. Entries the prefix filter skips, names the track has no group for
/// and group 0 give nothing.
pub(super) fn switches<'a>(list: impl IntoIterator<Item = &'a str>, groups: &SceneryGroups) -> Vec<Switch> {
    let mut out = Vec::new();
    for entry in list {
        let Some(parsed) = parse_barrier_entry(entry) else { continue };
        let Some(group) = groups.named(parsed.name) else {
            log::debug!("race barriers: no scenery group is named {}", parsed.name);
            continue;
        };
        if group.number == 0 {
            continue;
        }
        out.push(Switch { group: group.number, turned: parsed.star });
    }
    out
}

/// What a race changed, so its end can put every group back as it was (the engine clears every group at the end and
/// switches the stage's gates and the door on again, which leaves the same set).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(in crate::scenes::world) struct RaceWalls {
    /// Each group the race switched, with its state before (`Some(turned)` on, `None` off), in switch order.
    before: Vec<(u16, Option<bool>)>,
}

impl RaceWalls {
    /// How many groups the race switched.
    pub fn len(&self) -> usize {
        self.before.len()
    }

    /// Whether the race switched no group.
    pub fn is_empty(&self) -> bool {
        self.before.is_empty()
    }
}

/// Applies `switches` to `groups` through `switch`, remembering each group's state before its first switch.
fn raise(switches: &[Switch], groups: &SceneryGroups, mut switch: impl FnMut(u16, Option<bool>)) -> RaceWalls {
    let mut walls = RaceWalls::default();
    for s in switches {
        if !walls.before.iter().any(|(group, _)| *group == s.group) {
            walls.before.push((s.group, groups.state(s.group)));
        }
        switch(s.group, Some(s.turned));
    }
    walls
}

impl WorldScene {
    /// Switches on the groups of a race's `Barriers` list, on top of what is on (the region gates stay closed).
    ///
    /// The engine also flags the roads that cross the barrier lines, sparing the roads at the race's start. The
    /// rewrite clears the road flags for the race instead: with its crossing test 15 of 215 race courses lose their
    /// racer route, so the exact test is open (`docs/specs/race-barriers.md` section 5).
    pub(super) fn raise_race_walls(&mut self, list: &[String]) -> RaceWalls {
        let switches = switches(list.iter().map(String::as_str), &self.scenery_groups);
        let mut applied = Vec::with_capacity(switches.len());
        let walls = raise(&switches, &self.scenery_groups, |group, state| applied.push((group, state)));
        for (group, state) in applied {
            self.switch_scenery_group(group, state);
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.mark_road_barriers(&[]);
        }
        if !walls.is_empty() {
            log::info!("race: {} scenery group(s) of walls switched on", walls.len());
        }
        walls
    }

    /// Puts every group a race switched back as it was before the race, and the free-roam road flags with them.
    pub(super) fn lower_race_walls(&mut self, walls: RaceWalls) {
        for (group, state) in walls.before.into_iter().rev() {
            self.switch_scenery_group(group, state);
        }
        self.refresh_road_barriers();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_scenery::SceneryGroup;

    fn group(name: &str, number: u16) -> SceneryGroup {
        SceneryGroup {
            hash: blackbox_hash::bstring_hash(name),
            number,
            flag: 1,
            drive_through: false,
            override_indexes: Vec::new(),
        }
    }

    fn table() -> SceneryGroups {
        SceneryGroups::new(vec![
            group("BARRIER_SPLINE_516", 247),
            group("BARRIER_SPLINE_523", 270),
            group("SCENERY_GROUP_DOOR", 71),
            group("BARRIER_LOCKED_13", 200),
        ])
    }

    #[test]
    fn a_list_names_groups_by_hash_and_the_star_turns_them() {
        let list = ["*BARRIER_SPLINE_516", "BARRIER_SPLINE_523", "SCENERY_GROUP_DOOR"];
        assert_eq!(
            switches(list, &table()),
            [
                Switch { group: 247, turned: true },
                Switch { group: 270, turned: false },
                Switch { group: 71, turned: false },
            ]
        );
    }

    #[test]
    fn unknown_names_and_unaccepted_prefixes_are_skipped() {
        let list = ["BARRIER_SPLINE_9999", "XO_SOMETHING", "*", "barrier_spline_516"];
        assert!(switches(list, &table()).is_empty(), "the prefix test ignores case, the hash does not");
    }

    #[test]
    fn the_end_of_a_race_puts_back_what_was_on_before() {
        let mut groups = table();
        groups.switch(200, Some(false)); // a closed region gate
        groups.switch(71, Some(false)); // the door
        let list = [
            Switch { group: 247, turned: true },
            Switch { group: 200, turned: false },
            Switch { group: 71, turned: true },
            Switch { group: 247, turned: false },
        ];
        let mut live = groups.clone();
        let walls = raise(&list, &groups, |group, state| live.switch(group, state));
        assert_eq!(walls.len(), 3, "a group listed twice is remembered once");
        assert_eq!(live.state(247), Some(false), "the later entry wins, as in the engine");
        assert!(live.turned(71));
        for (group, state) in walls.before.into_iter().rev() {
            live.switch(group, state);
        }
        assert_eq!(live, groups, "the race wall goes, the gate and the door stay as they were");
    }
}
