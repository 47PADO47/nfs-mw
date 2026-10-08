//! The class parameters the car sound feeds the sample-layer modules. Spec: `docs/specs/engine-sound-aems.md` §2
//! and §3.

use blackbox_carsound::{EngineOutput, ShiftState};

use crate::audio::mixer::Levels;

/// Unity of the Q15 values of the sound system.
const Q15: f32 = 32767.0;

/// The number of parameters of the `CAR` class and of the `CAR_Sputter` class.
pub const CAR_PARAMS: usize = 26;
pub const SPUTTER_PARAMS: usize = 11;

/// The index of each `CAR` parameter the game sets.
mod car {
    pub const CLASS: usize = 0;
    pub const RPM: usize = 1;
    pub const TORQUE: usize = 3;
    pub const VOL_ENG: usize = 4;
    pub const VOL_EXH: usize = 5;
    pub const SPU_OR_EE: usize = 12;
    pub const PITCH_OFFSET: usize = 16;
    pub const MAX_RPM: usize = 25;
}

/// The index of each `CAR_Sputter` parameter the game sets.
mod sputter {
    pub const CLASS: usize = 0;
    pub const ID: usize = 1;
    pub const RPM: usize = 2;
    pub const VOL: usize = 3;
    pub const TORQUE: usize = 7;
    pub const ACCEL: usize = 9;
    pub const SHIFTING: usize = 10;
}

fn q15(fraction: f32) -> i32 {
    (fraction.clamp(0.0, 1.0) * Q15).round() as i32
}

/// The `CAR` parameters (the engine's sample layer) for this frame. `class` is the engine set's `CarID`.
pub fn engine(class: u32, out: &EngineOutput, levels: &Levels) -> [i32; CAR_PARAMS] {
    let mut p = [0; CAR_PARAMS];
    let layer = out.aems_volume * levels.engine_samples;
    // 0 is full volume, -32767 silence.
    let attenuation = q15(layer) - 32767;
    p[car::CLASS] = class as i32;
    p[car::RPM] = out.ginsu_frequency.round() as i32;
    p[car::TORQUE] = out.aems_torque.round() as i32;
    p[car::VOL_ENG] = attenuation;
    p[car::VOL_EXH] = attenuation;
    p[car::SPU_OR_EE] = 1;
    p[car::PITCH_OFFSET] = (levels.engine_pitch * 16383.0) as i32 - 0x3FFF;
    p[car::MAX_RPM] = q15(out.redline_sample_volume * levels.engine_samples);
    p
}

/// The `CAR_Sputter` parameters for this frame. `volume` is `engineaudio.Vol_Sputters`.
pub fn sputter(class: u32, id: u32, volume: i32, out: &EngineOutput, levels: &Levels) -> [i32; SPUTTER_PARAMS] {
    let mut p = [0; SPUTTER_PARAMS];
    p[sputter::CLASS] = class as i32;
    p[sputter::ID] = id as i32;
    p[sputter::RPM] = out.physics_rpm.round() as i32;
    p[sputter::VOL] = (levels.sparks * volume.clamp(0, 32767) as f32).round() as i32;
    p[sputter::TORQUE] = (out.physics_torque * 10.24).round() as i32;
    p[sputter::ACCEL] = i32::from(out.accelerating);
    p[sputter::SHIFTING] = i32::from(out.shift_state != ShiftState::None);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output() -> EngineOutput {
        EngineOutput {
            ginsu_frequency: 4321.4,
            aems_volume: 0.5,
            redline_sample_volume: 0.25,
            aems_torque: 512.4,
            physics_rpm: 6500.0,
            physics_torque: 50.0,
            accelerating: true,
            ..EngineOutput::default()
        }
    }

    #[test]
    fn the_engine_volume_is_an_attenuation_from_full() {
        let levels = Levels { engine_samples: 0.5, engine_pitch: 1.0, ..Levels::unmixed() };
        let p = engine(66, &output(), &levels);
        assert_eq!((p[0], p[1], p[3], p[12]), (66, 4321, 512, 1));
        assert_eq!(p[4], 8192 - 32767, "0.5 * 0.5 of full scale below full");
        assert_eq!(p[4], p[5], "the exhaust follows the engine");
        assert_eq!(p[25], 4096, "the redline sample: 0.25 * 0.5");
        assert_eq!(p[16], 0, "no pitch offset at unity");
    }

    #[test]
    fn full_volume_is_zero_attenuation_and_silence_is_minus_full() {
        let loud = EngineOutput { aems_volume: 1.0, ..output() };
        assert_eq!(engine(1, &loud, &Levels::unmixed())[4], 0);
        let quiet = EngineOutput { aems_volume: 0.0, ..output() };
        assert_eq!(engine(1, &quiet, &Levels::unmixed())[4], -32767);
    }

    #[test]
    fn the_pitch_offset_is_the_multiplier_around_unity() {
        let levels = Levels { engine_pitch: 1.25, ..Levels::unmixed() };
        assert_eq!(engine(1, &output(), &levels)[16], (1.25f32 * 16383.0) as i32 - 0x3FFF);
        let slow = Levels { engine_pitch: 0.5, ..Levels::unmixed() };
        assert!(engine(1, &output(), &slow)[16] < 0);
    }

    #[test]
    fn the_sputter_reads_the_physics_side() {
        let levels = Levels { sparks: 0.5, ..Levels::unmixed() };
        let p = sputter(66, 1, 30000, &output(), &levels);
        assert_eq!((p[0], p[1], p[2], p[3], p[7], p[9], p[10]), (66, 1, 6500, 15000, 512, 1, 0));
        let shifting = EngineOutput { shift_state: ShiftState::UpDisengage, accelerating: false, ..output() };
        let p = sputter(66, 1, -5, &shifting, &levels);
        assert_eq!((p[3], p[9], p[10]), (0, 0, 1), "a negative level is silent");
    }
}
