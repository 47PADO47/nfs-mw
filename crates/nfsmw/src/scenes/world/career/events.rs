//! Race and event entries in career mode (`docs/specs/career-blacklist.md`, `docs/specs/world-markers.md`).
//!
//! A race zone the car drives into is judged by the Blacklist: a free race is open once its bin has been entered
//! (sections 2.3 and 5.2), a boss race once its rival is unlocked or beaten (section 2.6). An open race raises the
//! autosave for an event start. Races are not built yet, so an open race still reaches the stub in `entry.rs`.

use nfsmw_career::blacklist::{BinRecord, BinState, Blacklist, EventResult, apply_event_result};
use nfsmw_career::phone::{BinMoment, bin_messages};
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

    /// A career race ended with the player in `place` (1 for first): the result goes to the Blacklist, which counts the
    /// race, opens the rival, enters the next bins and pays the race cash on a win, and the bin's race-win messages
    /// are queued (`docs/specs/career-blacklist.md` section 4.2, `docs/specs/career-phone.md` section 7).
    pub(in crate::scenes::world) fn career_race_finished(&mut self, event: &str, place: u8, cash_value: f32) {
        let phone = &mut self.career.phone;
        let outcome = self.career.handle.with(|career| {
            let result = EventResult {
                event: event.to_owned(),
                placing: place,
                cash_value,
                is_challenge: false,
                bin: 0,
                career: true,
                bounty_total: None,
                challenge_series_complete: false,
            };
            let outcome = apply_event_result(
                &mut career.settings,
                &mut career.blacklist,
                &mut career.milestones,
                &mut career.traps,
                &mut career.stable,
                &result,
            );
            let inbox = &mut career.settings.sms;
            if place == 1
                && let Some((bin, won)) = races_won_in_bin(&career.blacklist, event)
            {
                phone.queue_bin_messages(inbox, bin_messages(bin, BinMoment::RaceWins(won)));
            }
            if outcome.released_cars > 0 {
                phone.add_sms(inbox, RELEASE_SMS);
            }
            outcome
        });
        let Some(outcome) = outcome else { return };
        log::info!(
            "career: race {event} placed {place}: cash {}, bins done {:?}, bins entered {:?}, {} car(s) releasable",
            outcome.cash_paid,
            outcome.bins_done,
            outcome.bins_entered,
            outcome.released_cars
        );
    }

    /// The race is over and the world is free roam again: the roaming-start autosave runs, as `SetRoaming` does.
    pub(in crate::scenes::world) fn career_race_left(&mut self) {
        let inputs = AutosaveInputs {
            context: FrontEndContext::CAREER,
            node_id: None,
            boss_race: false,
            gameplay_idle: true,
            profile_loaded: true,
        };
        handle_autosave(&self.career.handle, self.career.profiles.as_ref(), Trigger::RoamingStart, inputs);
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

/// The SMS that tells the player an impounded car can be released (`docs/specs/career-economy-flows.md` section 7).
const RELEASE_SMS: u32 = 120;

/// The bin that lists `event` as a free race, and how many of its free races are won (the counter after this win).
fn races_won_in_bin(blacklist: &Blacklist, event: &str) -> Option<(u8, u8)> {
    let (record, kind) = find_race(blacklist, event)?;
    if kind != RaceKind::Free {
        return None;
    }
    let won = u8::try_from(record.races_won.len()).unwrap_or(u8::MAX);
    Some((record.def.number, won))
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
