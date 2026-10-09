//! A small deterministic random source (xorshift64*), so a seeded emitter always behaves the same.

#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // A zero state would stay zero.
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let bits = x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40;
        bits as f32 / (1u64 << 24) as f32
    }

    /// Uniform in `[0, range)`; `range` may be negative, then the result is in `(range, 0]`.
    pub fn below(&mut self, range: f32) -> f32 {
        self.unit() * range
    }

    pub fn bit(&mut self) -> bool {
        self.unit() >= 0.5
    }
}
