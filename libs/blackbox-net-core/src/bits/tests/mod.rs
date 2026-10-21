//! Seeded round-trip tests for the bit codecs. Each codec runs at least 10,000 random cases.

mod quant;
mod varint_ids;
mod writer_reader;

use crate::rng::SplitMix64;

/// The low `bits` bits set; all 64 when `bits` is 64.
fn mask(bits: u32) -> u64 {
    if bits >= 64 {
        return u64::MAX;
    }
    (1u64 << bits) - 1
}

/// A random value that fits in `bits` bits.
fn random_bits(rng: &mut SplitMix64, bits: u32) -> u64 {
    rng.next_u64() & mask(bits)
}

/// A random width in `0..=64`.
fn random_width(rng: &mut SplitMix64) -> u32 {
    rng.below(65) as u32
}
