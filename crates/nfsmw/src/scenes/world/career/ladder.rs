//! The unlock ladder of the side bins (the pursuit challenges `19.*`, the tollbooths `18.*` and the speed-trap bin
//! `21.*`). A race opens when its record says `InitiallyUnlocked` or when a won race lists it in `UnlockRaces`
//! (`docs/specs/career-scripts-events.md` section 6.2, `docs/formats/gameplay-races.md` section 9). The won events are
//! the session's `side_won` set, which is not in the save, so a new run starts the ladders again.

use std::collections::BTreeSet;

use nfsmw_data::gameplay::GameplayData;

use super::events::RaceEntry;
use crate::career::race_event_id;
use crate::scenes::world::WorldScene;

/// The bins whose races open through the ladder.
const SIDE_BINS: [&str; 3] = ["18.", "19.", "21."];

/// Whether the ladder holds this event shut. An event outside the side bins, or one the data does not know, is never
/// held.
pub(super) fn side_locked(data: &GameplayData, won: &BTreeSet<String>, event: &str) -> bool {
    if !SIDE_BINS.iter().any(|bin| event.starts_with(bin)) {
        return false;
    }
    let Some(race) = data.races.iter().find(|r| race_event_id(r) == event) else {
        return false;
    };
    if race.params.initially_unlocked {
        return false;
    }
    let opened_by = |other: &nfsmw_data::gameplay::Race| {
        won.contains(&race_event_id(other)) && other.params.unlock_races.contains(&race.key)
    };
    !data.races.iter().any(opened_by)
}

impl WorldScene {
    /// Whether the ladder holds this race zone's event shut.
    pub(in crate::scenes::world) fn side_race_locked(&self, event: &str) -> bool {
        let Some(data) = self.gameplay.as_ref() else { return false };
        side_locked(data, &self.career.side_won, event)
    }

    /// A race outside the Blacklist is open unless the ladder holds it.
    pub(super) fn side_entry(&self, event: &str) -> RaceEntry {
        match self.side_race_locked(event) {
            true => RaceEntry::Locked,
            false => RaceEntry::NotCareer,
        }
    }
}

#[cfg(test)]
mod tests {
    use nfsmw_data::gameplay::{Race, RaceParams};

    use super::*;

    fn race(event: &str, key: u32, open: bool, unlocks: &[u32]) -> Race {
        Race {
            key,
            event: Some(event.to_owned()),
            params: RaceParams { initially_unlocked: open, unlock_races: unlocks.to_vec(), ..RaceParams::default() },
            story: Default::default(),
            name: event.to_owned(),
            event_icon: String::new(),
            race_type: None,
            region: None,
            laps: 1,
            start: None,
            finish: None,
            checkpoints: Vec::new(),
            barriers: Vec::new(),
        }
    }

    fn data() -> GameplayData {
        GameplayData {
            races: vec![
                race("19.8.11", 1, true, &[2]),
                race("19.8.12", 2, false, &[3]),
                race("19.8.44", 3, false, &[]),
            ],
            ..GameplayData::default()
        }
    }

    #[test]
    fn a_start_is_open_and_a_win_opens_the_races_it_lists() {
        let data = data();
        let mut won = BTreeSet::new();
        assert!(!side_locked(&data, &won, "19.8.11"), "starts open");
        assert!(side_locked(&data, &won, "19.8.12"));
        assert!(side_locked(&data, &won, "19.8.44"));
        won.insert("19.8.11".to_owned());
        assert!(!side_locked(&data, &won, "19.8.12"), "opened by the first");
        assert!(side_locked(&data, &won, "19.8.44"), "needs the second");
    }

    #[test]
    fn other_bins_and_unknown_events_are_never_held() {
        let data = data();
        let won = BTreeSet::new();
        assert!(!side_locked(&data, &won, "15.1.1"));
        assert!(!side_locked(&data, &won, "19.9.99"));
    }
}
