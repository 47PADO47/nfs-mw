//! The 32-bit checksum of the save container (`docs/formats/career-save.md` section 1).
//!
//! A CRC with the polynomial `0x04C11DB7`, processed most significant bit first, with a nonstandard start: the
//! first four bytes are read big endian and complemented to seed the register, the remaining bytes are shifted
//! in one bit at a time (the register is **not** flushed with 32 zero bits afterwards), and the result is
//! complemented. The spec says this reproduces all three header words of all three saves. The descriptor of
//! every save (`0x10D`, `0xF848`) must give `0x28A9C0BB`; the tests check that.

/// The CRC polynomial.
const POLY: u32 = 0x04C1_1DB7;

/// The checksum of `data`, or `None` when it has fewer than four bytes (the seed needs four).
pub fn container_crc(data: &[u8]) -> Option<u32> {
    let (seed, rest) = data.split_first_chunk::<4>()?;
    let mut reg = !u32::from_be_bytes(*seed);
    for &byte in rest {
        for bit in (0..8).rev() {
            let top = reg >> 31;
            reg = (reg << 1) | u32::from((byte >> bit) & 1);
            if top != 0 {
                reg ^= POLY;
            }
        }
    }
    Some(!reg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_descriptor_of_a_profile_gives_the_constant_every_save_carries() {
        let mut descriptor = Vec::new();
        descriptor.extend_from_slice(&0x10Du32.to_le_bytes());
        descriptor.extend_from_slice(&0xF848u32.to_le_bytes());
        assert_eq!(container_crc(&descriptor), Some(0x28A9_C0BB));
    }

    #[test]
    fn fewer_than_four_bytes_have_no_checksum() {
        assert_eq!(container_crc(&[]), None);
        assert_eq!(container_crc(&[1, 2, 3]), None);
        assert!(container_crc(&[1, 2, 3, 4]).is_some());
    }

    #[test]
    fn four_bytes_are_just_the_complement_of_the_complemented_seed() {
        // With no remaining bytes the register is the complemented seed, and the result its complement.
        assert_eq!(container_crc(&[0x12, 0x34, 0x56, 0x78]), Some(0x1234_5678));
    }

    #[test]
    fn one_changed_bit_changes_the_checksum() {
        let a: Vec<u8> = (0..64).collect();
        let mut b = a.clone();
        b[40] ^= 0x10;
        assert_ne!(container_crc(&a), container_crc(&b));
    }
}
