//! Per-tire damage from a spike strip (spec: `docs/specs/ai-pursuit-roadblocks.md` section 6.3).
//!
//! A tire goes `None` → `Punctured` when it is punctured; a punctured tire has no effect on the physics
//! until its fuse runs out, then it is `Blown`. The fuse is 0.5 s, or 180 s on run-flat tires. A blown tire
//! brakes fully, releases its handbrake and keeps 30 % of its traction.

/// Seconds from a puncture to a blow-out.
pub const BLOW_DELAY: f32 = 0.5;
/// Seconds from a puncture to a blow-out on run-flat tires.
pub const RUN_FLAT_BLOW_DELAY: f32 = 180.0;
/// The traction a blown tire keeps, as a share of a sound one.
pub const BLOWN_TRACTION: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TireState {
    None,
    /// Punctured; the value is the seconds left before the blow-out.
    Punctured(f32),
    Blown,
}

/// The four tires' damage (wheel order: front left, front right, rear left, rear right).
#[derive(Debug, Clone, PartialEq)]
pub struct TireDamage {
    states: [TireState; 4],
    /// Run-flat tires: a puncture takes [`RUN_FLAT_BLOW_DELAY`] to blow.
    pub run_flat: bool,
}

impl Default for TireDamage {
    fn default() -> Self {
        Self { states: [TireState::None; 4], run_flat: false }
    }
}

impl TireDamage {
    pub fn state(&self, wheel: usize) -> TireState {
        self.states.get(wheel).copied().unwrap_or(TireState::None)
    }

    /// Puncture `wheel`; only a sound tire can be punctured. Returns whether it was punctured now.
    pub fn puncture(&mut self, wheel: usize) -> bool {
        let delay = if self.run_flat { RUN_FLAT_BLOW_DELAY } else { BLOW_DELAY };
        match self.states.get_mut(wheel) {
            Some(state @ TireState::None) => {
                *state = TireState::Punctured(delay);
                true
            }
            _ => false,
        }
    }

    /// Advance the fuses by `dt` seconds; punctured tires past their fuse are blown.
    pub fn advance(&mut self, dt: f32) {
        for state in &mut self.states {
            if let TireState::Punctured(left) = *state {
                let left = left - dt;
                *state = if left <= 0.0 { TireState::Blown } else { TireState::Punctured(left) };
            }
        }
    }

    /// Whether `wheel` is blown: the state that changes the physics.
    pub fn is_blown(&self, wheel: usize) -> bool {
        self.state(wheel) == TireState::Blown
    }

    /// Clear every tire (a repair).
    pub fn reset(&mut self) {
        self.states = [TireState::None; 4];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_punctured_tire_blows_after_its_fuse() {
        let mut t = TireDamage::default();
        assert!(t.puncture(1));
        assert!(!t.puncture(1), "only a sound tire can be punctured");
        t.advance(0.3);
        assert!(!t.is_blown(1), "still punctured");
        t.advance(0.3);
        assert!(t.is_blown(1), "blown once the 0.5 s fuse is gone");
        assert!(!t.is_blown(0), "the other tires are sound");
    }

    #[test]
    fn run_flat_tires_take_three_minutes() {
        let mut t = TireDamage { run_flat: true, ..Default::default() };
        t.puncture(2);
        t.advance(60.0);
        assert!(!t.is_blown(2));
        t.advance(121.0);
        assert!(t.is_blown(2));
    }

    #[test]
    fn reset_repairs_every_tire() {
        let mut t = TireDamage::default();
        t.puncture(0);
        t.advance(1.0);
        t.reset();
        assert_eq!(t, TireDamage::default());
    }

    #[test]
    fn out_of_range_wheels_are_ignored() {
        let mut t = TireDamage::default();
        assert!(!t.puncture(4));
        assert_eq!(t.state(9), TireState::None);
    }
}
