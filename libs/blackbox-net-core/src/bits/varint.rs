//! Helpers shared by the writer and the reader for varints and zigzag integers.

/// Number of groups a canonical varint of `value` takes when each group holds `group_bits` data bits.
///
/// `group_bits` must be in `1..=63`; the caller checks that.
pub(super) fn group_count(value: u64, group_bits: u32) -> u32 {
    let needed = 64 - value.leading_zeros();
    if needed == 0 {
        return 1;
    }
    needed.div_ceil(group_bits)
}

/// The most groups a 64-bit value can take with `group_bits` data bits per group.
pub(super) fn max_groups(group_bits: u32) -> u32 {
    64u32.div_ceil(group_bits)
}

/// The mask that keeps the `group_bits` data bits of one group.
pub(super) fn data_mask(group_bits: u32) -> u64 {
    (1u64 << group_bits) - 1
}

/// Zigzag encoding: maps signed values to unsigned so small magnitudes stay small.
pub(super) fn zigzag_encode(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

/// Inverse of [`zigzag_encode`].
pub(super) fn zigzag_decode(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}
