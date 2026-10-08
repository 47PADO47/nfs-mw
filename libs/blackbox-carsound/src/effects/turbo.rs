//! The turbo and supercharger: the spool loop and the blow-off. Spec: `docs/specs/engine-sound-effects.md` §4.

use super::{LoopId, SoundCommand, SoundRef};
use crate::engine::EngineOutput;
use crate::math::q15_gain;
use crate::rng::Rng;
use crate::tuning::TurboTuning;

/// Throttle (percent) above which the turbo spools.
const SPOOL_THROTTLE: f32 = 20.0;
/// Seconds the spool volume holds at a peak, and how long the duck after it lasts.
const HOLD_SECONDS: f32 = 1.0;
const DUCK_SECONDS: f32 = 2.0;
/// The duck takes the volume to `cos(DUCK_DEPTH * 90 deg)` (307.2 / 512).
const DUCK_DEPTH: f32 = 0.6;
/// The blow-off sample ids above 75 % spool.
const BIG_SPOOL: f32 = 0.75;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    None,
    Spooling,
    Blowoff,
}

#[derive(Debug, Clone)]
pub(crate) struct TurboFx {
    tuning: TurboTuning,
    rng: Rng,
    state: State,
    charge: f32,
    spool: f32,
    peak_for: f32,
    blowoff_left: f32,
    stopped: bool,
}

impl TurboFx {
    pub fn new(tuning: TurboTuning, seed: u32) -> Self {
        Self {
            tuning,
            rng: Rng::new(seed ^ 0x7B0F),
            state: State::None,
            charge: 0.0,
            spool: 0.0,
            peak_for: 0.0,
            blowoff_left: 0.0,
            stopped: false,
        }
    }

    /// One tick: the charge follows the throttle (`EngTorque` in percent) and the revs.
    pub fn tick(&mut self, engine: &EngineOutput, out: &mut Vec<SoundCommand>) {
        let t = &self.tuning;
        let charge_time = t.charge_time.max(1.0);
        let torque = engine.eng_torque;
        let turbo = if torque > SPOOL_THROTTLE { torque * 0.01 } else { 0.0 };
        if turbo > 0.01 {
            self.charge += turbo;
        } else {
            self.charge -= t.leak_rate;
        }
        let rpm_scale = ((engine.physics_rpm - 1000.0) * 0.01).clamp(0.0, 1.0) * 0.6 + 0.4;
        let limit = charge_time * rpm_scale;
        self.charge = self.charge.clamp(0.0, charge_time.min(limit));
        self.spool = self.charge / charge_time;

        // The duck after a peak: holds for a second, then falls.
        if self.charge >= limit - 1e-3 && torque > SPOOL_THROTTLE {
            self.peak_for += crate::engine::TICK_SECONDS;
        } else {
            self.peak_for = 0.0;
        }

        match self.state {
            State::None if torque > SPOOL_THROTTLE => self.state = State::Spooling,
            State::Spooling if torque < SPOOL_THROTTLE => self.blow_off(out),
            State::Blowoff => {
                self.blowoff_left -= crate::engine::TICK_SECONDS;
                if self.blowoff_left <= 0.0 {
                    self.state = State::None;
                }
            }
            _ => {}
        }
    }

    fn blow_off(&mut self, out: &mut Vec<SoundCommand>) {
        self.state = State::Blowoff;
        self.blowoff_left = self.tuning.blowoff_seconds;
        let spool = self.spool;
        self.charge = 0.0;
        self.peak_for = 0.0;
        let (id, level) = match spool > BIG_SPOOL {
            true => (1 + self.rng.below(2) as u8, self.tuning.blowoff_volume[1]),
            false => (0, self.tuning.blowoff_volume[0]),
        };
        let volume = q15_gain(level as f32) * spool;
        if volume > 0.0 {
            out.push(SoundCommand::Play { sound: SoundRef::TurboBlowoff(id), volume, pitch: 1.0 });
        }
    }

    /// The spool loop (once per update).
    pub fn emit(&mut self, out: &mut Vec<SoundCommand>) {
        let base = q15_gain(self.tuning.spool_volume as f32);
        let held = (self.peak_for - HOLD_SECONDS).max(0.0);
        let duck = (DUCK_DEPTH * (held / DUCK_SECONDS).min(1.0) * std::f32::consts::FRAC_PI_2).cos();
        let volume = base * duck * self.spool;
        if base <= 0.0 || self.spool <= 0.0 {
            if !std::mem::replace(&mut self.stopped, true) {
                out.push(SoundCommand::StopLoop(LoopId::Turbo));
            }
            return;
        }
        self.stopped = false;
        out.push(SoundCommand::SetLoop {
            id: LoopId::Turbo,
            sound: SoundRef::TurboSpool,
            volume,
            pitch: 0.8 + 0.4 * self.spool,
        });
    }
}
