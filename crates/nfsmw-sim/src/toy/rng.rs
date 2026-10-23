//! The toy world's own random stream: SplitMix64, seeded, never the wall clock or a global source.

use crate::hash::mix64;

const GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToyRng {
    state: u64,
}

impl ToyRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GAMMA);
        mix64(self.state)
    }

    /// A float in `[0, 1)` with 24 random bits.
    pub fn next_unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }

    /// A float in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_splitmix64_reference_stream() {
        let mut rng = ToyRng::new(0);
        assert_eq!(rng.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(rng.next_u64(), 0x6e78_9e6a_a1b9_65f4);
    }

    #[test]
    fn units_stay_in_range() {
        let mut rng = ToyRng::new(7);
        for _ in 0..10_000 {
            let v = rng.next_unit();
            assert!((0.0..1.0).contains(&v));
            let r = rng.range(-3.0, 5.0);
            assert!((-3.0..5.0).contains(&r));
        }
    }
}
