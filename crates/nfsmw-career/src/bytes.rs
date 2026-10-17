//! Little-endian field access on slices whose length the caller already checked.

pub(crate) fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

pub(crate) fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

pub(crate) fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_at(b, o))
}

pub(crate) fn put_u16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

pub(crate) fn put_u32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

pub(crate) fn put_f32(b: &mut [u8], o: usize, v: f32) {
    put_u32(b, o, v.to_bits());
}

/// Reads `N` bytes at `o` as an array.
pub(crate) fn array_at<const N: usize>(b: &[u8], o: usize) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(&b[o..o + N]);
    out
}

/// Reads eight little-endian u16 counters at `o`.
pub(crate) fn counters_at(b: &[u8], o: usize) -> [u16; 8] {
    let mut out = [0u16; 8];
    for (i, c) in out.iter_mut().enumerate() {
        *c = u16_at(b, o + 2 * i);
    }
    out
}

/// Writes eight little-endian u16 counters at `o`.
pub(crate) fn put_counters(b: &mut [u8], o: usize, counters: &[u16; 8]) {
    for (i, c) in counters.iter().enumerate() {
        put_u16(b, o + 2 * i, *c);
    }
}
