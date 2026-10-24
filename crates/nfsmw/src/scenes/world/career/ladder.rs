//! The unlock ladder of the side bins (the pursuit challenges `19.*`, the tollbooths `18.*` and the speed-trap bin
//! `21.*`). A race opens when its record says `InitiallyUnlocked` or when a won race lists it in `UnlockRaces`
//! (`docs/specs/career-scripts-events.md` section 6.2, `docs/formats/gameplay-races.md` section 9). The won events are
//! the `side_won` set. A win is also written to the race record of the event (the won-in-career flag, as the Blacklist's
//! races are), and the set is read back from the records when the world takes the career, so the ladders keep their place.

use std::collections::BTreeSet;

use nfsmw_career::settings::NodeStates;
use nfsmw_data::gameplay::GameplayData;

use super::events::RaceEntry;
use crate::career::race_event_id;
use crate::scenes::world::WorldScene;

/// The bins whose races open through the ladder.
const SIDE_BINS: [&str; 3] = ["18.", "19.", "21."];

/// Whether the ladder holds this event shut. An event outside the side bins, or one the data does not know, is never
/// held.
pub(super) fn side_locked(data: &GameplayData, won: &BTreeSet<String>, event: &str) -> bool {
    if !is_side_event(event) {
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

/// The side-bin events of `data` whose race record says they were won in the career.
pub(super) fn side_events_won(data: &GameplayData, nodes: &NodeStates) -> BTreeSet<String> {
    data.races
        .iter()
        .map(race_event_id)
        .filter(|id| is_side_event(id))
        .filter(|id| nodes.get(id).is_some_and(|state| state.won_in_career()))
        .collect()
}

/// Whether the event belongs to a side bin, whose races open through the ladder.
pub(super) fn is_side_event(event: &str) -> bool {
    SIDE_BINS.iter().any(|bin| event.starts_with(bin))
}

impl WorldScene {
    /// Takes the won events of the side bins from the career's race records (the world just took the career).
    pub(super) fn seed_side_won(&mut self) {
        let Some(data) = self.gameplay.clone() else { return };
        let won = self.career.handle.with(|career| side_events_won(&data, &career.settings.nodes()));
        self.career.side_won = won.unwrap_or_default();
    }

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
    fn the_won_events_are_read_back_from_the_race_records() {
        let data = data();
        let mut nodes = NodeStates::default();
        nodes.set_won_in_career("19.8.11").unwrap();
        nodes.set_won_in_career("1.2.3").unwrap();
        let won = side_events_won(&data, &nodes);
        assert_eq!(won.into_iter().collect::<Vec<_>>(), ["19.8.11"], "only side-bin events the data knows");
    }

    #[test]
    fn other_bins_and_unknown_events_are_never_held() {
        let data = data();
        let won = BTreeSet::new();
        assert!(!side_locked(&data, &won, "15.1.1"));
        assert!(!side_locked(&data, &won, "19.9.99"));
    }
}
