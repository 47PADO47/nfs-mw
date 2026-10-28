//! The wind of the 2012 game: the loops that the player's speed drives. Spec: `docs/formats/chameleon-sound-graph.md`,
//! "The sound of speed".
//!
//! `SOUND/GLOBAL.BNDL` holds a sequence driven by `LocalPlayer.GetPlayerSpeed` with dozens of items: mixer items that set the
//! gain of other channels, and wave items that are the wind, loops in windows of the speed. The shared sound bundle has two
//! more sequences with that driver (the prop collision, and a smaller one that also reads the throttle and `EngineOn`), so
//! the wind is the one with the most wave items.

use crate::graphs::SoundGraphs;
use crate::sequence::{ItemKind, Sequence};

/// The live value of the player's speed.
pub const PLAYER_SPEED: &str = "LocalPlayer.GetPlayerSpeed";

impl SoundGraphs {
    /// The sequence of the wind: of those driven by the player's speed, the one with the most wave items. `None` when no
    /// sequence has any (a car bundle has none).
    pub fn wind_sequence(&self) -> Option<&Sequence> {
        let waves =
            |sequence: &Sequence| sequence.items.iter().filter(|item| matches!(item.kind, ItemKind::Wave(_))).count();
        self.driven_by(PLAYER_SPEED).filter(|sequence| waves(sequence) > 0).max_by_key(|sequence| waves(sequence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::{Item, WaveItem};
    use crate::value::{Fade, Modulator};

    fn wave_item() -> Item {
        Item {
            lo: 0.0,
            hi: 1.0,
            fade_in: Fade { shape: 1.0, span: 0.0 },
            fade_out: Fade { shape: 1.0, span: 0.0 },
            kind: ItemKind::Wave(WaveItem {
                waves: vec![1],
                mixer: 2,
                gain: Modulator::default(),
                pitch: Modulator::default(),
                tracks_driver: false,
                random: false,
            }),
        }
    }

    fn sequence(id: u64, driver: &str, waves: usize) -> Sequence {
        Sequence { id, driver: driver.into(), lo: 0.0, hi: 180.0, items: vec![wave_item(); waves] }
    }

    #[test]
    fn the_wind_is_the_speed_sequence_with_the_most_loops() {
        let graphs = SoundGraphs {
            sequences: vec![
                sequence(1, "LocalPlayer.GetPlayerSpeed", 3),
                sequence(2, "LocalPlayer.GetPlayerSpeed", 15),
                sequence(3, "RaceCars.Entities[1].GetSpeedMph", 40),
            ],
            ..SoundGraphs::default()
        };
        assert_eq!(graphs.wind_sequence().map(|s| s.id), Some(2));
    }

    #[test]
    fn a_bundle_without_a_speed_sequence_has_no_wind() {
        let graphs = SoundGraphs { sequences: vec![sequence(1, "GetPlayerAudibleRPM", 5)], ..SoundGraphs::default() };
        assert!(graphs.wind_sequence().is_none());
        assert!(SoundGraphs::default().wind_sequence().is_none());
    }
}
