//! Independent PC emitter clocks; no fixed-rate Xenon dispatch.

use super::particle::uniform;
use glam::{Quat, Vec3};
use nfsmw_data::vehicle_effects::PcEmitter;

#[derive(Clone, Copy)]
enum Phase {
    NotStarted,
    Delay(f32),
    On(f32),
    Off(f32),
    Continuous,
    Done,
}

pub(super) struct PcSource {
    pub profile: PcEmitter,
    pub point: Vec3,
    pub rotation: Quat,
    pub owner: Vec3,
    pub intensity: f32,
    pub forced: bool,
    phase: Phase,
    carry: f32,
    seed: u64,
}

impl PcSource {
    pub fn new(
        profile: PcEmitter,
        point: Vec3,
        rotation: Quat,
        owner: Vec3,
        intensity: f32,
        forced: bool,
        seed: u64,
    ) -> Self {
        Self { profile, point, rotation, owner, intensity, forced, phase: Phase::NotStarted, carry: 0.0, seed }
    }

    fn start(&mut self, rollover: f32) {
        let p = self.profile;
        self.phase = Phase::Continuous;
        if (self.forced || p.one_shot || p.off > 0.0 || p.off_variance > 0.0) && (p.on > 0.0 || p.on_variance > 0.0) {
            self.phase = Phase::On((p.on + uniform(self.seed, 1) * p.on_variance + rollover).max(0.0));
        }
        self.seed = self.seed.wrapping_add(1);
    }

    pub fn done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }

    pub fn births(&mut self, dt: f32) -> usize {
        let p = self.profile;
        let mut active = match self.phase {
            Phase::Done => return 0,
            Phase::NotStarted => {
                let delay = match p.random_delay {
                    true => p.delay * uniform(self.seed, 0),
                    false => p.delay,
                };
                if delay > 0.0 {
                    self.phase = Phase::Delay(delay - dt);
                    return 0;
                }
                self.start(-dt);
                dt
            }
            Phase::Delay(left) | Phase::Off(left) => {
                let remainder = left - dt;
                if remainder > 0.0 {
                    self.phase = match self.phase {
                        Phase::Delay(_) => Phase::Delay(remainder),
                        _ => Phase::Off(remainder),
                    };
                    return 0;
                }
                self.start(remainder);
                dt + remainder
            }
            Phase::On(left) => {
                self.phase = Phase::On(left - dt);
                if left > dt {
                    return self.count(dt);
                }
                self.phase = match self.forced || p.one_shot {
                    true => Phase::Done,
                    false => Phase::Off(p.off + uniform(self.seed, 2) * p.off_variance + left - dt),
                };
                self.seed = self.seed.wrapping_add(1);
                dt + left
            }
            Phase::Continuous => dt,
        };
        if matches!(self.phase, Phase::Continuous) && (self.forced || p.one_shot) {
            active = p.life * (1.0 - p.life_variance);
            self.phase = Phase::Done;
        }
        self.count(active)
    }

    fn count(&mut self, active: f32) -> usize {
        let p = self.profile;
        if self.intensity <= 0.0 || self.intensity < p.intensity[0] || self.intensity > p.intensity[1] {
            return 0;
        }
        let amount = (active * p.rate * self.intensity * (1.0 - p.rate_variance)).clamp(0.0, 1024.0);
        let whole = amount as usize;
        if whole > 0 {
            return whole;
        }
        self.carry += amount;
        if self.carry <= 1.0 {
            return 0;
        }
        self.carry -= 1.0;
        1
    }
}
