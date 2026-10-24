//! Which race starts the career keeps shut, kept from one frame to the next.
//!
//! The map icons and the discs on the road ask for every race's lock each frame (`WorldScene::race_start_blips`,
//! `shut_race_zones`). The answer is a function of the Blacklist's bin states, the side bins' won events and the race
//! data, which change on a race result, a load or a stage change, not every frame. So the answers are kept by race name
//! and dropped when the [`Key`] of those inputs changes. The key is read from the career each frame (one lock, fifteen
//! bin lookups), so a change anywhere in the career is seen without every mutation site having to announce it.

use std::collections::HashMap;

use nfsmw_career::blacklist::BinState;

use super::events::{LAST_BIN, RaceEntry, race_entry};
use crate::career::event_id;
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
        let locked = self.race_locked_now(name);
        self.career.locks.borrow_mut().remember(name, locked);
        locked
    }

    /// [`Self::career_race_locked`] worked out from the career.
    fn race_locked_now(&self, name: &str) -> bool {
        let event = event_id(name);
        let Some(blacklisted) =
            self.career.handle.with(|career| race_entry(&career.blacklist, &event) == RaceEntry::Locked)
        else {
            return false;
        };
        blacklisted || self.side_race_locked(&event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
