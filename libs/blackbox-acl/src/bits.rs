//! Reading the buffer: little-endian words for the headers, a most-significant-bit-first stream for the animated data.

use crate::error::AclError;

/// A little-endian `u32` at `at`.
pub(crate) fn u32_at(bytes: &[u8], at: usize) -> Result<u32, AclError> {
    let end = at.checked_add(4).ok_or(AclError::Truncated(at))?;
    let word = bytes.get(at..end).ok_or(AclError::Truncated(at))?;
    Ok(u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
}

/// A little-endian `f32` at `at`.
pub(crate) fn f32_at(bytes: &[u8], at: usize) -> Result<f32, AclError> {
    Ok(f32::from_bits(u32_at(bytes, at)?))
}

/// A little-endian `u16` at `at`.
pub(crate) fn u16_at(bytes: &[u8], at: usize) -> Result<u16, AclError> {
    let end = at.checked_add(2).ok_or(AclError::Truncated(at))?;
    let word = bytes.get(at..end).ok_or(AclError::Truncated(at))?;
    Ok(u16::from_le_bytes([word[0], word[1]]))
}

/// One byte at `at`.
pub(crate) fn u8_at(bytes: &[u8], at: usize) -> Result<u8, AclError> {
    bytes.get(at).copied().ok_or(AclError::Truncated(at))
}

/// `count` bits (1 to 32) starting at bit `bit_offset` of a stream whose first bit is the most significant bit of the
/// first byte (ACL packs animated values big-endian).
pub(crate) fn read_bits(bytes: &[u8], bit_offset: usize, count: u32) -> Result<u32, AclError> {
    debug_assert!((1..=32).contains(&count));
    let first = bit_offset / 8;
    let shift = (bit_offset % 8) as u32;
    let needed = (shift + count).div_ceil(8) as usize;
    let end = first.checked_add(needed).ok_or(AclError::Truncated(first))?;
    let window = bytes.get(first..end).ok_or(AclError::Truncated(first))?;
    let mut acc: u64 = 0;
    for &b in window {
        acc = (acc << 8) | u64::from(b);
    }
    let right = (needed as u32) * 8 - shift - count;
    let mask = if count == 32 { u64::from(u32::MAX) } else { (1u64 << count) - 1 };
    Ok(((acc >> right) & mask) as u32)
}

/// A quantised component of `bits` bits as a float in `0..=1` (`bits` 1 to 23).
pub(crate) fn read_unorm(bytes: &[u8], bit_offset: usize, bits: u32) -> Result<f32, AclError> {
    let raw = read_bits(bytes, bit_offset, bits)?;
    let inv = 1.0 / ((1u32 << bits) - 1) as f32;
    Ok(raw as f32 * inv)
}

/// An uncompressed `f32` in the bit stream.
pub(crate) fn read_raw_f32(bytes: &[u8], bit_offset: usize) -> Result<f32, AclError> {
    Ok(f32::from_bits(read_bits(bytes, bit_offset, 32)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_are_read_most_significant_first() {
        let bytes = [0b1010_1100, 0b0101_0011, 0xFF, 0x00, 0x80];
        assert_eq!(read_bits(&bytes, 0, 4).unwrap(), 0b1010);
        assert_eq!(read_bits(&bytes, 4, 8).unwrap(), 0b1100_0101);
        assert_eq!(read_bits(&bytes, 3, 13).unwrap(), 0b0_1100_0101_0011);
        assert_eq!(read_bits(&bytes, 8, 32).unwrap(), 0x53FF_0080);
        assert_eq!(read_bits(&bytes, 7, 1).unwrap(), 0);
        assert!(read_bits(&bytes, 33, 8).is_err());
    }

    #[test]
    fn a_quantised_value_maps_to_zero_and_one() {
        assert_eq!(read_unorm(&[0xFF], 0, 8).unwrap(), 1.0);
        assert_eq!(read_unorm(&[0x00], 0, 8).unwrap(), 0.0);
        assert!((read_unorm(&[0b1000_0000], 0, 1).unwrap() - 1.0).abs() < 1e-7);
    }
}
