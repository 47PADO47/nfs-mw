//! Shift clunks, sweeteners and the brake mash. Spec: `docs/specs/engine-sound-effects.md` §1.6.

use super::{SoundCommand, SoundRef};
use crate::engine::{EngineOutput, ShiftDirection, TICK_SECONDS};
use crate::input::CarInput;
use crate::math::{q15_gain, ramp};
use crate::tuning::CarSoundTuning;

/// RPM (sound scale) from which the up-shift sweeteners play.
const SWEETENER_RPM: f32 = 7000.0;
/// Seconds between two brake mashes.
const MASH_GAP: f32 = 1.0;
/// Speed above which the brake mash plays, miles per hour.
const MASH_SPEED_MPH: f32 = 5.0;

#[derive(Debug, Clone)]
pub(crate) struct ShiftFx {
    up_volume: f32,
    down_volume: f32,
    sweet_volume: f32,
    is_local_player: bool,
    time: f32,
    last_mash: f32,
    brake_was_full: bool,
}

impl ShiftFx {
    pub fn new(tuning: &CarSoundTuning) -> Self {
        Self {
            up_volume: q15_gain(tuning.shift.up_volume as f32),
            down_volume: q15_gain(tuning.shift.down_volume as f32),
            sweet_volume: q15_gain(tuning.engine.shift_sweet_volume as f32),
            is_local_player: tuning.is_local_player,
            time: 0.0,
            last_mash: -MASH_GAP,
            brake_was_full: false,
        }
    }

    /// The sweetener's level at an RPM: 26000/32767 at 4000 and below, full at 7000 and above.
    fn sweetener_volume(&self, rpm: f32) -> f32 {
        self.sweet_volume * ramp(rpm, 4000.0, 7000.0, 26000.0 / 32767.0, 1.0)
    }

    /// The one-shots of the engine mix's events (once per update).
    pub fn events(&mut self, engine: &EngineOutput, out: &mut Vec<SoundCommand>) {
        let events = &engine.events;
        if let Some(direction) = events.gear_clunk {
            // Louder with the revs: cos((1 - s) * 90 deg), s = 0.1 + 0.9 * clamp((rpm - 1500) / 6500).
            let s = 0.1 + 0.9 * ((engine.physics_rpm - 1500.0) / 6500.0).clamp(0.0, 1.0);
            let level = ((1.0 - s) * std::f32::consts::FRAC_PI_2).cos();
            let up = direction == ShiftDirection::Up;
            let base = if up { self.up_volume } else { self.down_volume };
            out.push(SoundCommand::Play { sound: SoundRef::GearClunk { up }, volume: base * level, pitch: 1.0 });
        }
        let rpm = engine.physics_rpm;
        let sweetener = |id: u8, rpm: f32, out: &mut Vec<SoundCommand>| {
            let volume = self.sweetener_volume(rpm);
            if volume > 0.0 {
                out.push(SoundCommand::Play { sound: SoundRef::Sweetener(id), volume, pitch: 1.0 });
            }
        };
        if events.disengage_sweetener && rpm >= SWEETENER_RPM {
            sweetener(0, rpm, out);
        }
        if events.engage_sweetener && engine.rpm_at_shift >= SWEETENER_RPM {
            sweetener(1, engine.rpm_at_shift, out);
        }
        if events.accel_sweetener {
            sweetener(1, rpm, out);
        }
        if events.engine_off_sweetener {
            sweetener(0, rpm, out);
        }
    }

    /// The brake mash: the pedal reaching full from below, not too soon after the last, while moving.
    pub fn tick(&mut self, input: &CarInput, out: &mut Vec<SoundCommand>) {
        self.time += TICK_SECONDS;
        let full = input.brake >= 0.999;
        let pressed = full && !self.brake_was_full;
        self.brake_was_full = full;
        let moving = input.speed_mph() > MASH_SPEED_MPH;
        if self.is_local_player && pressed && moving && self.time - self.last_mash > MASH_GAP {
            self.last_mash = self.time;
            out.push(SoundCommand::Play { sound: SoundRef::BrakeMash, volume: 1.0, pitch: 1.0 });
        }
    }
}
