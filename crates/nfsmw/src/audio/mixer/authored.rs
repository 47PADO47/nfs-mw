//! Raw map words for authored effects; no legacy loop-volume multiplication.
use super::{
    CarMixer,
    ids::{object, player_object},
};
use crate::audio::car::{transmission::TransmissionLevels, tyres::TyreLevels};

impl CarMixer {
    pub(in crate::audio) fn tyres(&self) -> TyreLevels {
        let raw = |slot| i32::from(self.mixer.raw_slot(player_object(object::SKIDS), slot).unwrap_or(0));
        TyreLevels {
            forward: raw(5),
            side: raw(7),
            backward: raw(6),
            azimuth: raw(1),
            pitch_offset: (self.mixer.pitch(player_object(object::SKIDS), 8).unwrap_or(1.0) * 4096.0) as i32,
            wet: raw(9),
        }
    }
    pub(in crate::audio) fn transmission(&self) -> TransmissionLevels {
        let engine = if self.dual { object::ENGINE_DUAL } else { object::ENGINE_SINGLE };
        let raw = |slot| i32::from(self.mixer.raw_slot(player_object(engine), slot).unwrap_or(0));
        TransmissionLevels {
            volume: raw(3),
            azimuth: raw(0),
            pitch_offset: ((self.mixer.pitch(player_object(engine), 4).unwrap_or(1.0) * 4096.0) as i32)
                .clamp(-16383, 16383),
            low_pass: self.mixer.filter(player_object(engine), 5).unwrap_or(25000.0) as i32,
            wet: self.environment_values.aems_wet,
            dry: self.environment_values.aems_dry,
        }
    }
}
