//! The AttribSys name hash: Bob Jenkins' `lookup2` ("hash()", 1996, public domain) with
//! initval `0xABCDEF00`.
//!
//! Classes, fields, collections, types and vault dependencies are all keyed by this 32-bit hash
//! of their name. It is not the `bStringHash` used by bChunk data. Written from Jenkins' public
//! domain description; `docs/formats/attributes.md` ("Name hash") records how it was checked.

/// The initval AttribSys passes to `lookup2`.
pub const VLT_HASH_INIT: u32 = 0xABCD_EF00;

/// Golden ratio constant `lookup2` seeds `a` and `b` with.
const GOLDEN_RATIO: u32 = 0x9E37_79B9;

/// Hash of a name, as AttribSys keys it. `const`, so keys can be compile-time constants:
/// `const PVEHICLE: u32 = vlt_hash("pvehicle");`.
pub const fn vlt_hash(name: &str) -> u32 {
    lookup2(name.as_bytes(), VLT_HASH_INIT)
}

/// Bob Jenkins' `lookup2` hash of `key` with the given initval.
pub const fn lookup2(key: &[u8], initval: u32) -> u32 {
    let (mut a, mut b, mut c) = (GOLDEN_RATIO, GOLDEN_RATIO, initval);
    let mut i = 0;
    while key.len() - i >= 12 {
        a = a.wrapping_add(word(key, i));
        b = b.wrapping_add(word(key, i + 4));
        c = c.wrapping_add(word(key, i + 8));
        (a, b, c) = mix(a, b, c);
        i += 12;
    }
    // The last 0..=11 bytes. `c`'s low byte is taken by the length, so its bytes start at bit 8.
    c = c.wrapping_add(key.len() as u32);
    let mut j = 0;
    while i + j < key.len() {
        let byte = key[i + j] as u32;
        match j {
            0..=3 => a = a.wrapping_add(byte << (8 * j)),
            4..=7 => b = b.wrapping_add(byte << (8 * (j - 4))),
            _ => c = c.wrapping_add(byte << (8 * (j - 7))),
        }
        j += 1;
    }
    mix(a, b, c).2
}

/// Little-endian `u32` at `i` (the caller guarantees 4 bytes).
const fn word(key: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([key[i], key[i + 1], key[i + 2], key[i + 3]])
}

const fn mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 13);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 8);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 13);
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 12);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 16);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 5);
    a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 3);
    b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 10);
    c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 15);
    (a, b, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_names() {
        // Class and type keys measured in NFS: Most Wanted's attributes.bin.
        assert_eq!(vlt_hash("pvehicle"), 0x4A97_EC8F);
        assert_eq!(vlt_hash("gameplay"), 0x5CEA_9D46);
        assert_eq!(vlt_hash("default"), 0xEEC2_271A);
        assert_eq!(vlt_hash("EA::Reflection::Float"), 0x3C16_EC5E); // 21 bytes: one block + tail
        assert_eq!(vlt_hash("Attrib::Types::Vector4"), 0x34FD_E6BB);
        assert_eq!(vlt_hash("Attrib::ClassLoadData"), 0x5E97_0CBC);
        assert_eq!(vlt_hash("Attrib::CollectionLoadData"), 0x8E11_2EB7);
        assert_eq!(vlt_hash("Attrib::DatabaseLoadData"), 0xCBBC_628F);
    }

    #[test]
    fn block_boundaries() {
        // 12 and 24 bytes: whole blocks, empty tail; 16 bytes: one block + 4. Values from the
        // decomp's symbols/vlt.txt and the DepN chunk of attributes.bin.
        assert_eq!(vlt_hash("_Array_ANGLE"), 0x1097_70CF);
        assert_eq!(vlt_hash("Attrib::DatabaseLoadData"), 0xCBBC_628F);
        assert_eq!(vlt_hash("_Array_Amplitude"), 0x966D_915E);
        assert_eq!(vlt_hash("db.vlt"), 0x1822_8ADE);
        assert_eq!(vlt_hash(""), 0x82FC_1624);
        // Compile-time evaluation works.
        const KEY: u32 = vlt_hash("pvehicle");
        assert_eq!(KEY, 0x4A97_EC8F);
    }
}
