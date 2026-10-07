//! Name hashes used throughout the NFS: Most Wanted data files.
//!
//! Solids, textures, scenery and language strings are linked by name hash rather
//! than by pointer. See `docs/formats/models.md` ("Name hashing").

/// The engine-wide name hash: `h = h * 33 + byte`, seeded with `0xFFFFFFFF`.
///
/// Verified against every solid name in the install (see `docs/formats/models.md`).
pub const fn bstring_hash(name: &str) -> u32 {
    bstring_hash_bytes(name.as_bytes())
}

/// [`bstring_hash`] over raw bytes (names in the files are 8-bit, NUL-terminated).
pub const fn bstring_hash_bytes(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0xFFFF_FFFF;
    let mut i = 0;
    while i < bytes.len() {
        h = h.wrapping_mul(33).wrapping_add(bytes[i] as u32);
        i += 1;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_seed() {
        assert_eq!(bstring_hash(""), 0xFFFF_FFFF);
    }

    #[test]
    fn matches_reference_formula() {
        // Computed by hand from the formula: ((0xFFFFFFFF * 33) + 'A') mod 2^32.
        assert_eq!(bstring_hash("A"), 0xFFFF_FFFFu32.wrapping_mul(33).wrapping_add(0x41));
    }

    #[test]
    fn known_texture_name() {
        // From CARS/BMWM3GTR/TEXTURES.BIN: the TPK key 0x33CCB33D belongs to "BMWM3GTR_BADGING".
        assert_eq!(bstring_hash("BMWM3GTR_BADGING"), 0x33CC_B33D);
    }
}
