//! A stable 64-bit hash of world state, written by hand so it has no dependency and means the same thing on every platform.
//!
//! [`StateHasher`] is FNV-1a over a byte stream with a SplitMix64-style finaliser. Every multi-byte value is fed as little
//! endian bytes, floats by bit pattern (with `-0.0` folded into `0.0` and every NaN folded into one quiet NaN), and slices
//! with their length first, so two different shapes of state never feed the same bytes. [`StateHasher::raw`] is plain
//! FNV-1a; [`StateHasher::finish`] adds the finaliser so that one changed bit flips about half of the output bits.
//!
//! This is a change detector for determinism tests and desync checks, not a security hash.

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The bit pattern every NaN is hashed as.
const CANONICAL_NAN: u32 = 0x7fc0_0000;

/// The SplitMix64 output function: a bijective mix of 64 bits. Also the step of the toy world's random stream.
pub fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The bits a float is hashed as: equal values (and all NaNs) give equal bits.
pub fn f32_bits(v: f32) -> u32 {
    if v.is_nan() {
        return CANONICAL_NAN;
    }
    if v == 0.0 {
        return 0;
    }
    v.to_bits()
}

/// An incremental hasher of world state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateHasher {
    state: u64,
}

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHasher {
    pub fn new() -> Self {
        Self { state: FNV_OFFSET }
    }

    /// Feed raw bytes, one FNV-1a round each.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= u64::from(b);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    pub fn write_u8(&mut self, v: u8) {
        self.write_bytes(&[v]);
    }

    pub fn write_bool(&mut self, v: bool) {
        self.write_u8(u8::from(v));
    }

    pub fn write_u16(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.write_bytes(&v.to_le_bytes());
    }

    /// Feed a float by bit pattern; `-0.0` hashes as `0.0` and every NaN hashes alike.
    pub fn write_f32(&mut self, v: f32) {
        self.write_u32(f32_bits(v));
    }

    /// Feed a length then each float. The length keeps `[a]` + `[b, c]` apart from `[a, b]` + `[c]`.
    pub fn write_f32_slice(&mut self, values: &[f32]) {
        self.write_u32(values.len() as u32);
        for &v in values {
            self.write_f32(v);
        }
    }

    /// Feed a length then each integer.
    pub fn write_u32_slice(&mut self, values: &[u32]) {
        self.write_u32(values.len() as u32);
        for &v in values {
            self.write_u32(v);
        }
    }

    /// Feed a length then the bytes.
    pub fn write_byte_slice(&mut self, bytes: &[u8]) {
        self.write_u32(bytes.len() as u32);
        self.write_bytes(bytes);
    }

    /// The plain FNV-1a state, without the finaliser.
    pub fn raw(&self) -> u64 {
        self.state
    }

    /// The hash: the FNV-1a state through the SplitMix64 output function.
    pub fn finish(&self) -> u64 {
        mix64(self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_of(bytes: &[u8]) -> u64 {
        let mut h = StateHasher::new();
        h.write_bytes(bytes);
        h.raw()
    }

    #[test]
    fn matches_the_published_fnv1a_vectors() {
        assert_eq!(raw_of(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(raw_of(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(raw_of(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn integers_feed_little_endian_bytes() {
        let mut a = StateHasher::new();
        a.write_u32(0x0102_0304);
        assert_eq!(a.raw(), raw_of(&[4, 3, 2, 1]));
        let mut b = StateHasher::new();
        b.write_u16(0x0102);
        b.write_u64(1);
        assert_eq!(b.raw(), raw_of(&[2, 1, 1, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn negative_zero_and_nan_are_normalised() {
        assert_eq!(f32_bits(-0.0), f32_bits(0.0));
        assert_eq!(f32_bits(f32::NAN), f32_bits(-f32::NAN));
        assert_eq!(f32_bits(f32::from_bits(0x7fc0_0001)), f32_bits(f32::NAN));
        assert_ne!(f32_bits(1.0), f32_bits(-1.0));
        assert_ne!(f32_bits(f32::INFINITY), f32_bits(f32::NAN));
        assert_eq!(f32_bits(1.5), 1.5f32.to_bits());
    }

    #[test]
    fn slices_carry_their_length() {
        let mut a = StateHasher::new();
        a.write_f32_slice(&[1.0]);
        a.write_f32_slice(&[2.0, 3.0]);
        let mut b = StateHasher::new();
        b.write_f32_slice(&[1.0, 2.0]);
        b.write_f32_slice(&[3.0]);
        assert_ne!(a.finish(), b.finish());
    }

    #[test]
    fn finisher_spreads_a_single_bit() {
        let mut a = StateHasher::new();
        a.write_u32(0);
        let mut b = StateHasher::new();
        b.write_u32(1);
        let flipped = (a.finish() ^ b.finish()).count_ones();
        assert!((16..=48).contains(&flipped), "{flipped} bits differ");
    }

    #[test]
    fn mix64_is_the_splitmix_output_function() {
        assert_eq!(mix64(0), 0);
        assert_eq!(mix64(0x9e37_79b9_7f4a_7c15), 0xe220_a839_7b1d_cdaf);
    }

    #[test]
    fn default_equals_new() {
        assert_eq!(StateHasher::default(), StateHasher::new());
    }
}
