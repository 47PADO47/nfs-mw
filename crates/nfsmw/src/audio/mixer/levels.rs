//! What the mixer map says about the car's sounds this frame. Spec: `docs/specs/car-sound-mixer.md` §3 and §4.

use blackbox_mixmap::Mixer;

use super::ids::{object, player_object, slot};

/// The slot levels are relative; how loud the original's full scale is, is the sound system's business. All
/// gains are multiplied by this so the engine (about -9.6 dB in the map) stays as loud as it was before the maps
/// were read.
pub const MAKEUP: f32 = 2.25;

/// Gains (1 = unchanged; the makeup is included) and pitch ratios for the sounds of the car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levels {
    pub engine_volume: f32,
    pub engine_pitch: f32,
    pub clunk_up: f32,
    pub clunk_down: f32,
    pub sweet_engage: f32,
    pub sweet_disengage: f32,
    pub sweet_accelerate: f32,
    pub sweet_engine_off: f32,
    pub whine: f32,
    pub brake_mash: f32,
    pub turbo_spool: f32,
    pub turbo_blowoff_first: f32,
    pub turbo_blowoff_other: f32,
    pub nitrous: f32,
    pub purge: f32,
    pub nitrous_pitch: f32,
    pub skid_forward: f32,
    pub skid_back: f32,
    pub skid_side: f32,
    pub skid_pitch: f32,
    /// By the loop value of the surface.
    pub road: [f32; 9],
    pub wind: f32,
    pub wind_pitch: f32,
    pub landing: f32,
}

impl Levels {
    /// The levels when there is no map: every effect at its generated volume, the road and the wind at the
    /// guesses used before the maps were read.
    pub fn unmixed() -> Self {
        Self {
            engine_volume: 1.0,
            engine_pitch: 1.0,
            clunk_up: 1.0,
            clunk_down: 1.0,
            sweet_engage: 1.0,
            sweet_disengage: 1.0,
            sweet_accelerate: 1.0,
            sweet_engine_off: 1.0,
            whine: 1.0,
            brake_mash: 1.0,
            turbo_spool: 1.0,
            turbo_blowoff_first: 1.0,
            turbo_blowoff_other: 1.0,
            nitrous: 1.0,
            purge: 1.0,
            nitrous_pitch: 1.0,
            skid_forward: 1.0,
            skid_back: 1.0,
            skid_side: 1.0,
            skid_pitch: 1.0,
            road: [0.35; 9],
            wind: 0.4,
            wind_pitch: 1.0,
            landing: 1.0,
        }
    }

    /// The levels the mixer computed in its last frame.
    pub fn read(m: &Mixer, dual: bool) -> Self {
        let gain = |obj: u8, slot: usize| m.volume(player_object(obj), slot).unwrap_or(0.0) * MAKEUP;
        let pitch = |obj: u8, slot: usize| m.pitch(player_object(obj), slot).unwrap_or(1.0);
        let engine = if dual { object::ENGINE_DUAL } else { object::ENGINE_SINGLE };
        let mut road = [0.0; 9];
        for (value, level) in road.iter_mut().enumerate() {
            *level = gain(object::ROAD, slot::ROAD[value]);
        }
        let wind = (gain(object::WIND, slot::WIND_LEFT) + gain(object::WIND, slot::WIND_RIGHT)) / 2.0;
        Self {
            engine_volume: gain(engine, slot::ENGINE_GINSU),
            engine_pitch: pitch(engine, slot::ENGINE_PITCH),
            clunk_up: gain(object::SHIFT, slot::CLUNK_UP),
            clunk_down: gain(object::SHIFT, slot::CLUNK_DOWN),
            sweet_engage: gain(object::SHIFT, slot::SWEET_ENGAGE),
            sweet_disengage: gain(object::SHIFT, slot::SWEET_DISENGAGE),
            sweet_accelerate: gain(object::SHIFT, slot::SWEET_ACCELERATE),
            sweet_engine_off: gain(object::SHIFT, slot::SWEET_ENGINE_OFF),
            whine: gain(object::SHIFT, slot::WHINE),
            brake_mash: gain(object::SHIFT, slot::BRAKE_MASH),
            turbo_spool: gain(object::TURBO, slot::TURBO_SPOOL),
            turbo_blowoff_first: gain(object::TURBO, slot::TURBO_BLOWOFF_FIRST),
            turbo_blowoff_other: gain(object::TURBO, slot::TURBO_BLOWOFF_OTHER),
            nitrous: gain(object::NITROUS, slot::NITROUS),
            purge: gain(object::NITROUS, slot::PURGE),
            nitrous_pitch: pitch(object::NITROUS, slot::NITROUS_PITCH),
            skid_forward: gain(object::SKIDS, slot::SKID_FORWARD),
            skid_back: gain(object::SKIDS, slot::SKID_BACK),
            skid_side: gain(object::SKIDS, slot::SKID_SIDE),
            skid_pitch: pitch(object::SKIDS, slot::SKID_PITCH),
            road,
            wind,
            wind_pitch: pitch(object::WIND, slot::WIND_PITCH),
            landing: gain(object::BOTTOM_OUT, slot::LANDING),
        }
    }
}
