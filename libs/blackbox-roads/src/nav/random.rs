//! The random source a navigator draws junction exits from.

/// Uniform random numbers in `[0, 1)`; a seeded one makes replays match.
pub trait RandomSource {
    fn next_f32(&mut self) -> f32;

    /// A uniform index below `len` (which must be non-zero).
    fn index(&mut self, len: usize) -> usize {
        ((self.next_f32() * len as f32) as usize).min(len - 1)
    }
}

/// SplitMix64: small, fast, good enough for gameplay choices.
#[derive(Debug, Clone)]
pub struct SplitMix(pub u64);

impl RandomSource for SplitMix {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }
}
