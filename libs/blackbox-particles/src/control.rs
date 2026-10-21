//! Emission phases and PC boundary deltas; see `docs/specs/particle-emission-control.md`.
use crate::{EmitterSpec, Rng};

#[derive(Debug, Clone, Copy, Default)]
enum Phase {
    #[default]
    New,
    Delay,
    OnCycle,
    OffCycle,
    On,
    Off,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Control {
    phase: Phase,
    left: f32,
}

pub(crate) fn has_on_cycle(spec: &EmitterSpec, one_shot: bool) -> bool {
    (spec.on_cycle != 0.0 || spec.on_cycle_variance != 0.0)
        && (one_shot || spec.off_cycle != 0.0 || spec.off_cycle_variance != 0.0)
}

impl Control {
    pub fn finished(&self) -> bool {
        matches!(self.phase, Phase::Off)
    }

    pub fn emission_delta(&mut self, spec: &EmitterSpec, forced: bool, rng: &mut Rng, dt: f32) -> Option<f32> {
        let one_shot = forced || spec.one_shot;
        let on_cycle = has_on_cycle(spec, one_shot);
        let delay = match spec.start_delay_random {
            true => rng.below(spec.start_delay),
            false => spec.start_delay,
        };
        match self.phase {
            Phase::New if delay != 0.0 => {
                self.phase = Phase::Delay;
                self.left = delay - dt;
                None
            }
            Phase::New if on_cycle => {
                self.phase = Phase::OnCycle;
                self.left = (spec.on_cycle + rng.below(spec.on_cycle_variance) - dt).max(0.0);
                Some(dt)
            }
            Phase::New => {
                self.phase = Phase::On;
                Some(dt)
            }
            Phase::Delay => {
                self.left -= dt;
                if self.left > 0.0 {
                    return None;
                }
                let delta = dt + self.left;
                if on_cycle {
                    self.phase = Phase::OnCycle;
                    self.left += spec.on_cycle + rng.below(spec.on_cycle_variance);
                    return Some(delta);
                }
                self.phase = Phase::On;
                self.left = 0.0;
                Some(delta)
            }
            Phase::OnCycle => {
                let previous = self.left;
                self.left -= dt;
                if self.left > 0.0 {
                    return Some(dt);
                }
                if one_shot {
                    self.phase = Phase::Off;
                    self.left = 0.0;
                    return Some(dt + previous);
                }
                self.phase = Phase::OffCycle;
                self.left += spec.off_cycle + rng.below(spec.off_cycle_variance);
                Some(dt + previous)
            }
            Phase::OffCycle => {
                self.left -= dt;
                if self.left > 0.0 {
                    return None;
                }
                let delta = dt + self.left;
                self.phase = Phase::OnCycle;
                self.left += spec.on_cycle + rng.below(spec.on_cycle_variance);
                Some(delta)
            }
            Phase::On => Some(dt),
            Phase::Off => None,
        }
    }
}
