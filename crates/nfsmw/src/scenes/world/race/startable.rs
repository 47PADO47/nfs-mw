//! Which stored races the world can start. The race types the world runs are in `setup::runnable`; the NIS shell race
//! (16.8.1) is a story race of no racing, so it is startable whatever its type: its intro plays its NIS, and the race is
//! counted as won (`intro.rs`, `shell_outro`; `docs/specs/story-flow.md` section 1.3).

use nfsmw_data::gameplay::Race;

use super::setup;

/// Whether the world can start `race`: a race of a runnable type, or the NIS shell race.
pub fn can_start(race: &Race) -> bool {
    race.story.nis_shell || setup::runnable(race.race_type)
}

/// The opponents a race runs with: none for the NIS shell, which has no racing (`start.rs` plans the grid with them).
pub fn runs_opponents(race: &Race) -> bool {
    !race.story.nis_shell
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use nfsmw_data::gameplay::{Placement, RaceParams, RaceStory, RaceType};

    fn race(race_type: Option<RaceType>, nis_shell: bool) -> Race {
        Race {
            key: 7,
            name: "race_bin_16/16_nis_rog_razor".into(),
            event: None,
            event_icon: "p2p".into(),
            race_type,
            region: None,
            laps: 1,
            start: Some(Placement { position: Vec3::ZERO, heading_deg: 0.0 }),
            finish: Some(3),
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: RaceParams::default(),
            story: RaceStory { nis_shell, ..Default::default() },
        }
    }

    #[test]
    fn a_runnable_type_can_start() {
        assert!(can_start(&race(Some(RaceType::Sprint), false)));
        assert!(runs_opponents(&race(Some(RaceType::Sprint), false)));
    }

    #[test]
    fn the_nis_shell_starts_whatever_its_type_and_runs_no_opponents() {
        assert!(can_start(&race(None, true)));
        assert!(can_start(&race(Some(RaceType::CashGrab), true)));
        assert!(!runs_opponents(&race(None, true)));
    }

    #[test]
    fn a_race_of_an_unrunnable_type_without_the_shell_cannot_start() {
        assert!(!can_start(&race(Some(RaceType::CashGrab), false)));
        assert!(!can_start(&race(None, false)));
    }
}
