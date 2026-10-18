//! The region gates: which of the six `BARRIER_LOCKED_n` scenery groups are closed at which stage, how a
//! `Barriers` entry is read, and when the set is re-evaluated (`docs/specs/career-regions.md` sections 1 to 3).
//!
//! There is no region table in the data: the closed set comes from the `Barriers` list of the bin record of the
//! current stage. The rewrite should read that list ([`gates_in_list`]) so a modded bin works; [`closed_gates`] is
//! the documented fallback table and the test oracle.

/// The first and last gate number.
pub const FIRST_GATE: u8 = 10;
/// The last gate number.
pub const LAST_GATE: u8 = 15;

/// A set of region gates (numbers 10 to 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct GateSet(u16);

impl GateSet {
    /// No gate.
    pub const EMPTY: GateSet = GateSet(0);
    /// Gates 11, 12, 13, 14 and 15: the city while the stage is 13 or higher.
    pub const NORTH_WEST: GateSet = GateSet::of(&[11, 12, 13, 14, 15]);
    /// Gates 10, 12 and 13: the city at stages 12 to 9.
    pub const MIDDLE: GateSet = GateSet::of(&[10, 12, 13]);

    /// The set holding exactly these gate numbers; numbers outside 10 to 15 are ignored.
    pub const fn of(gates: &[u8]) -> GateSet {
        let mut bits = 0u16;
        let mut i = 0;
        while i < gates.len() {
            let g = gates[i];
            if g >= FIRST_GATE && g <= LAST_GATE {
                bits |= 1 << g;
            }
            i += 1;
        }
        GateSet(bits)
    }

    /// True when the gate is in the set.
    pub const fn contains(self, gate: u8) -> bool {
        gate >= FIRST_GATE && gate <= LAST_GATE && self.0 & (1 << gate) != 0
    }

    /// Adds a gate (numbers outside 10 to 15 are ignored).
    pub fn insert(&mut self, gate: u8) {
        self.0 |= GateSet::of(&[gate]).0;
    }

    /// The union of two sets.
    pub const fn union(self, other: GateSet) -> GateSet {
        GateSet(self.0 | other.0)
    }

    /// The gates in `self` that are not in `other`.
    pub const fn minus(self, other: GateSet) -> GateSet {
        GateSet(self.0 & !other.0)
    }

    /// True for the empty set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The gate numbers, ascending.
    pub fn numbers(self) -> Vec<u8> {
        (FIRST_GATE..=LAST_GATE).filter(|&g| self.contains(g)).collect()
    }
}

/// The closed gates of a stage: stage 13 and up (16 to 13) closes {11, 12, 13, 14, 15}; 12 to 9 closes {10, 12,
/// 13}; 8 to 1 closes none. A number that is no stage (above 16) has no bin record and closes none.
pub const fn closed_gates(stage: u8) -> GateSet {
    match stage {
        13..=16 => GateSet::NORTH_WEST,
        9..=12 => GateSet::MIDDLE,
        _ => GateSet::EMPTY,
    }
}

/// What a stage change does to the gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateChange {
    /// Gates that were closed and now are open.
    pub opened: GateSet,
    /// Gates that were open and now are closed (gate 10 closes when the stage drops from 13 to 12).
    pub closed: GateSet,
}

/// The difference between the closed sets of two stages. With `previous = None` (a world was just created) nothing
/// is "opened": the new set is simply closed.
pub fn apply_stage(previous: Option<u8>, new: u8) -> GateChange {
    let now = closed_gates(new);
    let Some(previous) = previous else {
        return GateChange { opened: GateSet::EMPTY, closed: now };
    };
    let before = closed_gates(previous);
    GateChange { opened: before.minus(now), closed: now.minus(before) }
}

/// One entry of a `Barriers` list, read the way the enable routine reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarrierEntry<'a> {
    /// The group name without the leading `*`.
    pub name: &'a str,
    /// The entry started with `*` (turn the pieces by 180 degrees; the meaning is unconfirmed).
    pub star: bool,
}

/// The name prefixes the enable routine accepts; the comparison ignores case. Any other entry is silently skipped.
pub const ACCEPTED_PREFIXES: [&str; 5] = ["BARRIER_", "BARRIERS_", "PLAYER_BARRIERS_", "SCENERY_GROUP_", "FREE_ROAM"];

/// Reads a `Barriers` entry: a leading `*` is removed and remembered, and an entry whose name does not start with
/// one of [`ACCEPTED_PREFIXES`] (case-insensitive) is skipped (`None`).
pub fn parse_barrier_entry(text: &str) -> Option<BarrierEntry<'_>> {
    let (name, star) = match text.strip_prefix('*') {
        Some(rest) => (rest, true),
        None => (text, false),
    };
    let accepted = ACCEPTED_PREFIXES
        .iter()
        .any(|prefix| name.get(..prefix.len()).is_some_and(|head| head.eq_ignore_ascii_case(prefix)));
    accepted.then_some(BarrierEntry { name, star })
}

/// The region gate number named by a group name: exactly `BARRIER_LOCKED_<n>` (the hash is case sensitive) with n
/// from 10 to 15.
pub fn gate_number(name: &str) -> Option<u8> {
    let n: u8 = name.strip_prefix("BARRIER_LOCKED_")?.parse().ok()?;
    (FIRST_GATE..=LAST_GATE).contains(&n).then_some(n)
}

/// The region gates a `Barriers` list closes, after the prefix filter. Order does not matter.
pub fn gates_in_list<'a>(entries: impl IntoIterator<Item = &'a str>) -> GateSet {
    let mut set = GateSet::EMPTY;
    for entry in entries {
        let Some(parsed) = parse_barrier_entry(entry) else { continue };
        if let Some(n) = gate_number(parsed.name) {
            set.insert(n);
        }
    }
    set
}

/// The state a group gets when it is enabled: bit 0 always, bit 1 when starred, bit 2 when the group drive-through
/// byte is set. Any non-zero state makes the group's collision instance count.
pub const fn group_state(star: bool, drive_through: bool) -> u8 {
    let star_bit = match star {
        true => 2,
        false => 0,
    };
    let drive_through_bit = match drive_through {
        true => 4,
        false => 0,
    };
    1 | star_bit | drive_through_bit
}

/// A collision query accepts an instance only if its group number is 0 or the group state byte is non-zero: a
/// closed gate blocks cars, an open one is ignored.
pub const fn instance_blocks(group_number: u16, state: u8) -> bool {
    group_number == 0 || state != 0
}

/// The scenery group number and piece count of each gate, from the install (`L2RA.BUN`).
pub const GATES: [(u8, u16, u8); 6] =
    [(10, 624, 3), (11, 389, 5), (12, 356, 7), (13, 200, 5), (14, 495, 7), (15, 496, 5)];

/// The scenery group number of gate `gate` (10 to 15), from [`GATES`].
pub fn group_of(gate: u8) -> Option<u16> {
    GATES.iter().find(|(number, _, _)| *number == gate).map(|(_, group, _)| *group)
}

/// The gates a challenge series race lists for its duration, whatever the stage: challenges 01 to 05 the five
/// north-west gates, challenges 06 to 15 gates {10, 12, 13}. Other numbers list none.
pub const fn challenge_gates(challenge: u8) -> GateSet {
    match challenge {
        1..=5 => GateSet::NORTH_WEST,
        6..=15 => GateSet::MIDDLE,
        _ => GateSet::EMPTY,
    }
}

/// The runtime state of the gates: the stored bin record, the "bin changed" byte, the enabled region gates and the
/// gates a running race added on top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RegionGates {
    /// The stage of the stored bin record, if one is stored.
    pub stored_stage: Option<u8>,
    /// The "bin changed" byte of the race status object, set by the enter-bin routine.
    pub bin_changed: bool,
    region: GateSet,
    race: GateSet,
}

impl RegionGates {
    /// No gate closed, nothing stored.
    pub fn new() -> RegionGates {
        RegionGates::default()
    }

    /// Every gate currently closed: the region gates and the ones a race listed.
    pub const fn closed(&self) -> GateSet {
        self.region.union(self.race)
    }

    /// The region gates alone.
    pub const fn region_gates(&self) -> GateSet {
        self.region
    }

    /// The race-status object is created with the world: with no event node current, the barriers are cleared and
    /// the list of the record of the stage byte is enabled. With an event node nothing happens here.
    pub fn world_created(&mut self, stage: u8, has_event_node: bool) {
        if has_event_node {
            return;
        }
        self.clear();
        self.enable_stage(stage);
    }

    /// The race-status object is reset (a routine of the world-loaded path): with no event node and a stored bin
    /// record, the list is enabled again without clearing.
    pub fn race_status_reset(&mut self, has_event_node: bool) {
        if has_event_node {
            return;
        }
        if let Some(stage) = self.stored_stage {
            self.region = closed_gates(stage);
        }
    }

    /// The enter-bin routine changed the stage: only a byte is set. Nothing is re-evaluated until the next
    /// roaming update.
    pub fn stage_changed(&mut self) {
        self.bin_changed = true;
    }

    /// One race-status update. In the roaming state (racing state 0) with the byte set, the byte is cleared and
    /// *set current bin* runs with the stage byte: clear (when a record was stored), look the record up, store it,
    /// enable its list. Returns the change of the closed region gates, `None` when nothing ran.
    pub fn roaming_update(&mut self, racing_state_zero: bool, stage: u8) -> Option<GateChange> {
        if !racing_state_zero || !self.bin_changed {
            return None;
        }
        self.bin_changed = false;
        let before = self.region;
        if self.stored_stage.is_some() {
            self.clear();
        }
        self.enable_stage(stage);
        Some(GateChange { opened: before.minus(self.region), closed: self.region.minus(before) })
    }

    /// A race starts: its own list is **added** on top. Nothing is cleared first, so the region gates stay closed
    /// during every race.
    pub fn race_started(&mut self, race_list: GateSet) {
        self.race = self.race.union(race_list);
    }

    /// The `basicrace` graph `done` state: `DisableBarriers` (everything off), then the stored record list is
    /// enabled again. Race barriers are gone and the region gates are back.
    pub fn race_done(&mut self) {
        self.clear();
        if let Some(stage) = self.stored_stage {
            self.region = closed_gates(stage);
        }
    }

    fn clear(&mut self) {
        self.region = GateSet::EMPTY;
        self.race = GateSet::EMPTY;
    }

    fn enable_stage(&mut self, stage: u8) {
        self.stored_stage = Some(stage);
        self.region = closed_gates(stage);
    }
}

#[cfg(test)]
mod tests;
