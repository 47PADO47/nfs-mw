//! Which race starts the career keeps shut, kept from one frame to the next.
//!
//! The map icons and the discs on the road ask for every race's lock each frame (`WorldScene::race_start_blips`,
//! `shut_race_zones`). The answer is a function of the Blacklist's bin states, the side bins' won events and the race
//! data, which change on a race result, a load or a stage change, not every frame. So the answers are kept by race name
//! and dropped when the [`Key`] of those inputs changes. The key is read from the career each frame (one lock, fifteen
//! bin lookups), so a change anywhere in the career is seen without every mutation site having to announce it.

use std::collections::{BTreeSet, HashMap};

use nfsmw_career::blacklist::{BinState, Blacklist};
use nfsmw_data::gameplay::{GameplayData, Race};

use super::events::{LAST_BIN, RaceEntry, race_entry};
use super::ladder::{is_side_event, side_locked};
use crate::career::race_event_id;
use crate::scenes::world::WorldScene;

/// The inputs of a lock the career can change: the state of each bin (0 not entered, 1 locked, 2 unlocked, 3 done) and
/// the number of side-bin events won.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    bins: [u8; LAST_BIN as usize],
    side_won: usize,
}

/// The locks worked out for the current [`Key`], by race name.
#[derive(Default)]
pub(in crate::scenes::world) struct RaceLocks {
    /// The key the entries belong to: `None` when nothing is kept, `Some(None)` outside career mode.
    key: Option<Option<Key>>,
    locked: HashMap<String, bool, foldhash::fast::FixedState>,
}

impl RaceLocks {
    /// Keeps the entries only while the career's inputs are `key`.
    fn rekey(&mut self, key: Option<Key>) {
        if self.key == Some(key) {
            return;
        }
        self.locked.clear();
        self.key = Some(key);
    }

    /// Drops every entry, for a change the key does not show (the won side events were replaced).
    pub(in crate::scenes::world) fn invalidate(&mut self) {
        self.key = None;
        self.locked.clear();
    }

    fn get(&self, name: &str) -> Option<bool> {
        self.locked.get(name).copied()
    }

    fn remember(&mut self, name: &str, locked: bool) {
        self.locked.insert(name.to_owned(), locked);
    }
}

fn bin_code(state: Option<BinState>) -> u8 {
    match state {
        None => 0,
        Some(BinState::RivalLocked) => 1,
        Some(BinState::RivalUnlocked) => 2,
        Some(BinState::Done) => 3,
    }
}

/// Installed roaming entries are authored engage targets. Stored event IDs distinguish reverse courses.
pub(super) fn roaming_entry(
    data: &GameplayData,
    blacklist: &Blacklist,
    side_won: &BTreeSet<String>,
    race: &Race,
) -> RaceEntry {
    if !data.engages.is_empty() && !data.engages.iter().any(|entry| entry.target == race.key) {
        return RaceEntry::Locked;
    }
    let event = race_event_id(race);
    match race_entry(blacklist, &event) {
        RaceEntry::NotCareer if is_side_event(&event) && !side_locked(data, side_won, &event) => RaceEntry::NotCareer,
        RaceEntry::NotCareer => RaceEntry::Locked,
        entry => entry,
    }
}

impl WorldScene {
    /// Ties the kept locks to the career as it is now. Called once a frame before the locks are asked for.
    pub(in crate::scenes::world) fn refresh_race_locks(&self) {
        let side_won = self.career.side_won.len();
        let key = self.career.handle.with(|career| {
            let mut bins = [0; LAST_BIN as usize];
            for (number, code) in (1..=LAST_BIN).zip(bins.iter_mut()) {
                *code = bin_code(career.blacklist.bin(number).and_then(|record| record.state));
            }
            Key { bins, side_won }
        });
        self.career.locks.borrow_mut().rekey(key);
    }

    /// Whether a race is one the Blacklist lists and has not opened yet: its start icon stays off the map. The answer is
    /// the one kept since the last [`Self::refresh_race_locks`] that saw the same career.
    pub(in crate::scenes::world) fn career_race_locked(&self, name: &str) -> bool {
        let kept = self.career.locks.borrow().get(name);
        if let Some(locked) = kept {
            return locked;
        }
        let locked = self.roaming_entry(name) == RaceEntry::Locked;
        self.career.locks.borrow_mut().remember(name, locked);
        locked
    }

    /// A rival's authored road entry uses a separate stock map icon type.
    pub(in crate::scenes::world) fn ordinary_race_marker(&self, name: &str) -> bool {
        !self.career_race_locked(name) && !(self.entry.zones.rivals.contains(name) && self.career.handle.is_active())
    }

    pub(super) fn roaming_entry(&self, name: &str) -> RaceEntry {
        self.career
            .handle
            .with(|career| {
                let Some(data) = self.gameplay.as_ref() else { return RaceEntry::Locked };
                let Some(race) = data.race_named(name) else { return RaceEntry::Locked };
                roaming_entry(data, &career.blacklist, &self.career.side_won, race)
            })
            .unwrap_or(RaceEntry::NotCareer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsmw_data::gameplay::{EngageMarker, RaceParams};

    fn key(side_won: usize) -> Option<Key> {
        Some(Key { bins: [0; LAST_BIN as usize], side_won })
    }

    #[test]
    fn an_answer_is_kept_while_the_key_is_the_same() {
        let mut locks = RaceLocks::default();
        locks.rekey(key(0));
        assert_eq!(locks.get("a"), None);
        locks.remember("a", true);
        locks.rekey(key(0));
        assert_eq!(locks.get("a"), Some(true));
    }

    #[test]
    fn a_new_key_or_an_invalidation_drops_the_answers() {
        let mut locks = RaceLocks::default();
        locks.rekey(key(0));
        locks.remember("a", true);
        locks.rekey(key(1));
        assert_eq!(locks.get("a"), None, "a win changes the key");
        locks.remember("a", false);
        locks.invalidate();
        assert_eq!(locks.get("a"), None, "an invalidation drops them");
        locks.remember("a", true);
        locks.rekey(None);
        assert_eq!(locks.get("a"), None, "leaving career mode drops them");
    }

    #[test]
    fn bin_states_have_distinct_codes() {
        let codes = [
            bin_code(None),
            bin_code(Some(BinState::RivalLocked)),
            bin_code(Some(BinState::RivalUnlocked)),
            bin_code(Some(BinState::Done)),
        ];
        assert_eq!(codes, [0, 1, 2, 3]);
    }

    #[test]
    fn authored_side_entries_keep_their_unlock_ladder() {
        use nfsmw_career::blacklist::BinDef;

        let blacklist = Blacklist::new(
            (1..=15)
                .map(|number| BinDef {
                    number,
                    races: Vec::new(),
                    races_needed: 0,
                    challenges_needed: 0,
                    bounty_needed: 0,
                    rival_events: Vec::new(),
                })
                .collect(),
            15,
        )
        .unwrap();
        let side = |key, event: &str, initially_unlocked, unlock_races| Race {
            key,
            name: format!("authored_bonus_{key}"),
            event_icon: String::new(),
            race_type: None,
            event: Some(event.to_owned()),
            region: None,
            laps: 1,
            start: None,
            finish: None,
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: RaceParams { initially_unlocked, unlock_races, ..Default::default() },
            story: Default::default(),
        };
        let data = GameplayData {
            races: vec![side(1, "21.1.1", true, vec![2]), side(2, "21.2.1", false, Vec::new())],
            engages: (1..=2).map(|target| EngageMarker { target, position: glam::Vec3::ZERO, radius: 4.0 }).collect(),
            ..Default::default()
        };
        let mut won = BTreeSet::new();
        assert_eq!(roaming_entry(&data, &blacklist, &won, &data.races[0]), RaceEntry::NotCareer);
        assert_eq!(roaming_entry(&data, &blacklist, &won, &data.races[1]), RaceEntry::Locked);
        won.insert("21.1.1".to_owned());
        assert_eq!(roaming_entry(&data, &blacklist, &won, &data.races[1]), RaceEntry::NotCareer);
    }
}
