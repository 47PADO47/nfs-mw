//! Little-endian field writers, the counterpart of `crate::bytes`.

pub(super) fn put_u16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

pub(super) fn put_u32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

pub(super) fn put_f32(b: &mut [u8], o: usize, v: f32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

pub(super) fn put_vec3(b: &mut [u8], o: usize, v: [f32; 3]) {
    for (i, f) in v.into_iter().enumerate() {
        put_f32(b, o + i * 4, f);
    }
}

/// `(hash, 0)` pairs: the entry format of the texture and light-material lists and of the pack's key table.
pub(super) fn hash_pairs(hashes: impl IntoIterator<Item = u32>) -> Vec<u8> {
    hashes.into_iter().flat_map(|h| [h.to_le_bytes(), [0; 4]]).flatten().collect()
}
