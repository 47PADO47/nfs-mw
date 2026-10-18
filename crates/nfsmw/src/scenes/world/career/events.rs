//! Race and event entries in career mode (`docs/specs/career-blacklist.md`, `docs/specs/world-markers.md`).
//!
//! A race zone the car drives into is judged by the Blacklist: a free race is open once its bin has been entered
//! (sections 2.3 and 5.2), a boss race once its rival is unlocked or beaten (section 2.6). An open race raises the
//! autosave for an event start. Races are not built yet, so an open race still reaches the stub in `entry.rs`.

use nfsmw_career::blacklist::{BinRecord, BinState, Blacklist};
use nfsmw_career::safehouse::FrontEndContext;
use nfsmw_career::savefile::Trigger;

use crate::career::{AutosaveInputs, event_id, handle_autosave};
use crate::scenes::world::WorldScene;

/// The Blacklist's bins, 1 to 15 (`FIRST_RIVAL_STAGE`).
const LAST_BIN: u8 = 15;

/// Whether a race zone can be entered in the career.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) enum RaceEntry {
    /// Not career mode, or the race is not one the Blacklist knows (a prologue, tollbooth or challenge race).
    NotCareer,
    /// The Blacklist has not opened the race yet.
    Locked,
    /// The race is open. The event-start autosave has been raised.
    Open,
}

/// Whether the Blacklist lists a race as a free race of its bin or as a rival's boss race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RaceKind {
    Free,
    Boss,
}

impl WorldScene {
    /// The career's answer to a race zone the car drove into, with its card up and Interact pressed. An open race
    /// raises the event-start autosave; the caller then shows the stub, or the locked card for a locked race.
    pub(in crate::scenes::world) fn career_enter_race(&mut self, name: &str) -> RaceEntry {
        let event = event_id(name);
        let Some(entry) = self.career.handle.with(|career| race_entry(&career.blacklist, &event)) else {
            return RaceEntry::NotCareer;
        };
        if entry == RaceEntry::Open {
            self.autosave_event_start(&event);
        }
        entry
    }

    /// The event-start autosave for an open race: the node is the race's event id. A boss race is flagged as one for
    /// the gate, which refuses it.
    fn autosave_event_start(&self, event: &str) {
        let boss = self.career.handle.with(|career| is_boss_race(&career.blacklist, event)).unwrap_or(false);
        let inputs = AutosaveInputs {
            context: FrontEndContext::CAREER,
            node_id: Some(event.to_owned()),
            boss_race: boss,
            // Entered from the roaming world, so the gameplay manager has no notification pending.
            gameplay_idle: true,
            profile_loaded: true,
        };
        let trigger = Trigger::EventStart { node: Some(event), race_flag: true };
        handle_autosave(&self.career.handle, self.career.profiles.as_ref(), trigger, inputs);
    }
}

/// Judges a race zone's event id against the Blacklist. Pure: the caller reads the career.
pub(super) fn race_entry(blacklist: &Blacklist, event: &str) -> RaceEntry {
    let Some((record, kind)) = find_race(blacklist, event) else {
        return RaceEntry::NotCareer;
    };
    if race_open(record, kind) {
        return RaceEntry::Open;
    }
    RaceEntry::Locked
}

/// Whether the event is a rival's boss race in the Blacklist. False for a free race or an unknown event.
pub(super) fn is_boss_race(blacklist: &Blacklist, event: &str) -> bool {
    matches!(find_race(blacklist, event), Some((_, RaceKind::Boss)))
}

/// The bin that lists the event, and whether it lists it as a free race or a boss race. A boss race is checked first,
/// so an event listed both ways counts as a boss race.
fn find_race<'a>(blacklist: &'a Blacklist, event: &str) -> Option<(&'a BinRecord, RaceKind)> {
    for number in 1..=LAST_BIN {
        let Some(record) = blacklist.bin(number) else {
            continue;
        };
        if record.def.rival_events.iter().any(|id| id == event) {
            return Some((record, RaceKind::Boss));
        }
        if record.def.races.iter().any(|id| id == event) {
            return Some((record, RaceKind::Free));
        }
    }
    None
}

/// Whether a race of this bin can be entered, from the bin's state.
///
/// A bin the walk has not entered has no state and opens nothing. A free race opens with its bin: the spec unlocks
/// only the races flagged `InitiallyUnlocked` on entry and the rest through other races' unlock lists, which the
/// Blacklist does not keep **[unconfirmed]**, so this opens every free race of an entered bin. A boss race opens
/// when the rival is unlocked, and stays open once the bin is done (section 2.6).
fn race_open(record: &BinRecord, kind: RaceKind) -> bool {
    let Some(state) = record.state else {
        return false;
    };
    match kind {
        RaceKind::Free => true,
        RaceKind::Boss => matches!(state, BinState::RivalUnlocked | BinState::Done),
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
