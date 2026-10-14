//! Raw authored parameters for the road and wind modules; Q15 gains and Q12 pitch.
use super::{
    CarMixer,
    ids::{object, player_object},
};

#[derive(Clone, Copy)]
pub(in crate::audio) struct AmbientLevels {
    pub wind: [i32; 6],
    pub road: [i32; 8],
    pub road_azimuth: [i32; 2],
    pub road_wet: i32,
}

impl Default for AmbientLevels {
    fn default() -> Self {
        Self { wind: [0, 0, 13107, 13107, 0, 4096], road: [11468; 8], road_azimuth: [0; 2], road_wet: 0 }
    }
}

impl CarMixer {
    pub(in crate::audio) fn ambient(&self) -> AmbientLevels {
        let raw = |object, slot| i32::from(self.mixer.raw_slot(player_object(object), slot).unwrap_or(0));
        let mut wind = std::array::from_fn(|i| raw(object::WIND, i));
        wind[5] = (self.mixer.pitch(player_object(object::WIND), 5).unwrap_or(1.0) * 4096.0) as i32;
        AmbientLevels {
            wind,
            road: [5, 6, 7, 8, 9, 10, 10, 11].map(|i| raw(object::ROAD, i)),
            road_azimuth: [0, 1].map(|i| raw(object::ROAD, i)),
            road_wet: raw(object::ROAD, 15),
        }
    }
}
