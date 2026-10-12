//! The one-shot sounds of the traffic cars: their horns on a crash and the whoosh of a near miss. The scene reports
//! them; the mixer plays them. Spec: `docs/specs/traffic-sound-events.md` §4.

use kira::sound::static_sound::StaticSoundData;

use super::volume::decibels;
use super::{Audio, Group};

/// The traffic car horns: `HornType` is the sound number in this bank (`docs/specs/ai-traffic-world.md` §3.2).
pub const HORN_BANK: &str = "IG_GLOBAL/TRAFFIC_MB.abk";
/// The near-miss whoosh bank. Which sound number is the whoosh is **[unconfirmed]**; 1 is a guess.
pub const WHOOSH_BANK: &str = "IG_GLOBAL/Stitch_Whoosh_MB.abk";
const WHOOSH_INDEX: usize = 1;

/// A one-shot the traffic cars asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrafficSound {
    /// A car's horn: `index` is its `HornType`.
    Horn { index: usize, volume: f32 },
    /// A traffic car passed the player close by.
    NearMiss { volume: f32 },
}

impl TrafficSound {
    /// The bank and sound number this one plays.
    pub fn source(self) -> (&'static str, usize) {
        match self {
            Self::Horn { index, .. } => (HORN_BANK, index),
            Self::NearMiss { .. } => (WHOOSH_BANK, WHOOSH_INDEX),
        }
    }

    /// The linear volume it plays at.
    pub fn volume(self) -> f32 {
        match self {
            Self::Horn { volume, .. } | Self::NearMiss { volume } => volume,
        }
    }
}

impl Audio {
    /// Plays a traffic one-shot. A sound that cannot be read is logged once and then dropped.
    pub fn play_traffic(&mut self, sound: TrafficSound) {
        let (bank, index) = sound.source();
        let data: StaticSoundData = match self.bank_sound(bank, index) {
            Ok(data) => data,
            Err(e) => {
                if self.missing.insert((bank.to_owned(), index)) {
                    log::warn!("no traffic sound {sound:?}: {e}");
                }
                return;
            }
        };
        let data = data.volume(decibels(sound.volume()));
        if let Err(e) = self.play(Group::Sfx, data) {
            log::debug!("traffic sound {sound:?}: {e}");
        }
    }
}
