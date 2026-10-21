//! The track list of a custom race: the races of the mode and region the player filtered, with the ones the career has not
//! opened marked as locked (`docs/specs/custom-race.md` section 3).

use std::sync::OnceLock;

use nfsmw_career::blacklist::{BinState, Blacklist};

use super::mode::Mode;
use super::region::Filter;
use crate::frontend::quick_race::{Board, Track};

/// A track of the list: its place on the board and whether the player cannot run it yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed {
    pub index: usize,
    pub locked: bool,
}

/// The environment variable that opens every track, as the original's `UnlockAllThings` does **[ours]**.
const UNLOCK_ALL: &str = "NFSMW_UNLOCK_ALL";

/// Whether every track is open (a developer's switch, read once).
pub fn unlock_all() -> bool {
    static ALL: OnceLock<bool> = OnceLock::new();
    *ALL.get_or_init(|| std::env::var_os(UNLOCK_ALL).is_some_and(|value| value != "0"))
}

/// What the career has opened of the races: the Blacklist position and the boss races of the rivals already unlocked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    /// The Blacklist position the career is at (16 before the first rival, 1 for the last).
    pub stage: u8,
    /// The events of the boss races of every bin at `rival_unlocked`: the bin's script unlocks them then, before the rival is
    /// beaten **[exe]** (`UnlockRace` from `rival_unlocked`, `docs/specs/career-blacklist.md` section 2.6).
    pub boss_races: Vec<String>,
}

impl Standing {
    /// A career at `stage` with no rival waiting to be challenged.
    pub fn at(stage: u8) -> Self {
        Self { stage, boss_races: Vec::new() }
    }

    /// The standing of a Blacklist.
    pub fn of(blacklist: &Blacklist) -> Self {
        let waiting = (1..=15)
            .filter_map(|number| blacklist.bin(number))
            .filter(|record| record.state == Some(BinState::RivalUnlocked))
            .flat_map(|record| record.def.rival_events.iter().cloned());
        Self { stage: blacklist.current_bin(), boss_races: waiting.collect() }
    }
}

/// The tracks of a mode and region, in the order the screen steps through them: the Blacklist bins from 15 down to 1, and
/// within a bin the order of the event ids **[decomp]** (`BuildPresetTrackList` walks the bins downwards). A race with no bin
/// from 1 to 15 is not listed: the prologue, the challenge series, the online events and the collector's events are not
/// custom races (the loop skips bins 16 to 20 and `IsEventAvailable` refuses 21 and 99 outside the collector's edition).
///
/// A track opens in the quick race when a script calls `UnlockRace` on it **[exe]** (flag 4 unless the race is
/// `NeverInQuickRace`, which the board leaves out). The bin script calls it for the races open from the start, for the boss races
/// at `rival_unlocked` and for the free races at `done`, which is when rival `b` is beaten and the stage has fallen below `b`;
/// the screen says "Unlock this track by beating <rival>, Blacklist member <b>" **[exe]**.
pub fn list(board: &Board, mode: Mode, filter: Filter, standing: &Standing, open_all: bool) -> Vec<Listed> {
    let mut tracks: Vec<Listed> = board
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.race_type == mode.race_type() && track.bin.is_some())
        .filter(|(_, track)| filter.admits(track.region))
        .map(|(index, track)| Listed { index, locked: !open_all && !is_open(track, standing) })
        .collect();
    tracks.sort_by_key(|listed| std::cmp::Reverse(board.tracks[listed.index].bin));
    tracks
}

/// Whether the career has opened the track.
pub fn is_open(track: &Track, standing: &Standing) -> bool {
    track.open_at_start
        || track.bin.is_some_and(|bin| bin > standing.stage)
        || standing.boss_races.contains(&track.event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::custom_race::Region;
    use nfsmw_data::gameplay::RaceType;

    fn track(name: &str, kind: RaceType, bin: Option<u8>, region: Region) -> Track {
        Track { race_type: kind, bin, region: Some(region), event: name.into(), ..Track::sample(name) }
    }

    fn board() -> Board {
        Board {
            tracks: vec![
                track("a", RaceType::Drag, Some(2), Region::City),
                track("b", RaceType::Drag, Some(9), Region::College),
                track("c", RaceType::Sprint, Some(9), Region::City),
                track("d", RaceType::Drag, None, Region::City),
                track("e", RaceType::Drag, Some(9), Region::City),
            ],
        }
    }

    fn names(board: &Board, listed: &[Listed]) -> Vec<String> {
        listed.iter().map(|l| board.tracks[l.index].name.clone()).collect()
    }

    #[test]
    fn a_mode_lists_its_races_with_a_bin_from_the_highest_bin_down() {
        let board = board();
        let all = list(&board, Mode::Drag, Filter::ALL, &Standing::at(16), false);
        assert_eq!(names(&board, &all), ["b", "e", "a"]);
    }

    #[test]
    fn the_region_filter_narrows_the_list() {
        let board = board();
        let city = list(&board, Mode::Drag, Filter::from_index(2), &Standing::at(16), false);
        assert_eq!(names(&board, &city), ["e", "a"]);
        assert!(list(&board, Mode::Drag, Filter::from_index(1), &Standing::at(16), false).is_empty());
    }

    #[test]
    fn a_track_opens_when_its_rival_is_beaten() {
        let board = board();
        let locked = |stage| {
            list(&board, Mode::Drag, Filter::ALL, &Standing::at(stage), false)
                .iter()
                .map(|l| l.locked)
                .collect::<Vec<_>>()
        };
        assert_eq!(locked(16), [true, true, true]);
        assert_eq!(locked(8), [false, false, true], "bin 9 is open once member 9 is beaten");
        assert_eq!(locked(1), [false, false, false]);
        assert!(list(&board, Mode::Drag, Filter::ALL, &Standing::at(16), true).iter().all(|l| !l.locked));
    }

    #[test]
    fn a_boss_race_opens_when_its_rival_is_unlocked() {
        let board = board();
        let standing = Standing { stage: 16, boss_races: vec![board.tracks[4].event.clone()] };
        let locked =
            list(&board, Mode::Drag, Filter::ALL, &standing, false).iter().map(|l| l.locked).collect::<Vec<_>>();
        assert_eq!(locked, [true, false, true], "only the boss race \"e\" is open before its rival is beaten");
    }
}
