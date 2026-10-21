//! The track list of a custom race: the races of the mode and region the player filtered, with the ones the career has not
//! opened marked as locked (`docs/specs/custom-race.md` section 3).

use std::sync::OnceLock;

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

/// The tracks of a mode and region, in the order the screen steps through them: the Blacklist bins from 15 down to 1, and
/// within a bin the order of the event ids **[decomp]** (`BuildPresetTrackList` walks the bins downwards). A race with no bin
/// from 1 to 15 is not listed: the prologue, the challenge series, the online events and the collector's events are not
/// custom races (the loop skips bins 16 to 20 and `IsEventAvailable` refuses 21 and 99 outside the collector's edition).
///
/// `stage` is the Blacklist position the career is at (16 before the first rival, 1 for the last). A track of bin `b` is
/// open once rival `b` is beaten, which is when the stage has fallen below `b`; a race open from the start is always open
/// **[ours]**: the screen says "Unlock this track by beating <rival>, Blacklist member <b>" **[exe]**, and the exe's
/// `IsQuickRaceUnlocked` was not traced further.
pub fn list(board: &Board, mode: Mode, filter: Filter, stage: u8, open_all: bool) -> Vec<Listed> {
    let mut tracks: Vec<Listed> = board
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.race_type == mode.race_type() && track.bin.is_some())
        .filter(|(_, track)| filter.admits(track.region))
        .map(|(index, track)| Listed { index, locked: !open_all && !is_open(track, stage) })
        .collect();
    tracks.sort_by_key(|listed| std::cmp::Reverse(board.tracks[listed.index].bin));
    tracks
}

/// Whether the career at `stage` has opened the track.
pub fn is_open(track: &Track, stage: u8) -> bool {
    track.open_at_start || track.bin.is_some_and(|bin| bin > stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::custom_race::Region;
    use nfsmw_data::gameplay::RaceType;

    fn track(name: &str, kind: RaceType, bin: Option<u8>, region: Region) -> Track {
        Track { race_type: kind, bin, region: Some(region), ..Track::sample(name) }
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
        let all = list(&board, Mode::Drag, Filter::ALL, 16, false);
        assert_eq!(names(&board, &all), ["b", "e", "a"]);
    }

    #[test]
    fn the_region_filter_narrows_the_list() {
        let board = board();
        let city = list(&board, Mode::Drag, Filter::from_index(2), 16, false);
        assert_eq!(names(&board, &city), ["e", "a"]);
        assert!(list(&board, Mode::Drag, Filter::from_index(1), 16, false).is_empty());
    }

    #[test]
    fn a_track_opens_when_its_rival_is_beaten() {
        let board = board();
        let locked =
            |stage| list(&board, Mode::Drag, Filter::ALL, stage, false).iter().map(|l| l.locked).collect::<Vec<_>>();
        assert_eq!(locked(16), [true, true, true]);
        assert_eq!(locked(8), [false, false, true], "bin 9 is open once member 9 is beaten");
        assert_eq!(locked(1), [false, false, false]);
        assert!(list(&board, Mode::Drag, Filter::ALL, 16, true).iter().all(|l| !l.locked));
    }
}
