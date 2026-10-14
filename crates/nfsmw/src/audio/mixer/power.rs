//! Power effects consume original mixer words, without fallback makeup gain.
use super::{
    CarMixer,
    ids::{object, player_object},
};
use crate::audio::car::power_effects::PowerLevels;

impl CarMixer {
    pub(in crate::audio) fn power(&self) -> PowerLevels {
        let raw = |object, slot| i32::from(self.mixer.raw_slot(player_object(object), slot).unwrap_or(0));
        PowerLevels {
            nitrous_volume: raw(object::NITROUS, 1),
            purge_volume: raw(object::NITROUS, 2),
            nitrous_azimuth: raw(object::NITROUS, 0),
            nitrous_pitch: (self.mixer.pitch(player_object(object::NITROUS), 3).unwrap_or(1.0) * 4096.0) as i32,
            nitrous_low_pass: self.mixer.filter(player_object(object::NITROUS), 4).unwrap_or(25000.0) as i32,
            shift_azimuth: raw(object::SHIFT, 0),
            shift_volume: [1, 2, 3, 4, 5, 6, 10].map(|slot| raw(object::SHIFT, slot)),
            turbo_azimuth: raw(object::TURBO, 0),
            turbo_volume: [1, 2, 3].map(|slot| raw(object::TURBO, slot)),
            whine_volume: raw(object::SHIFT, 8),
        }
    }
}
