//! Little-endian field access. Callers check lengths first.

pub fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

pub fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

pub fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

pub fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_bits(u32_at(b, o))
}

/// A NUL-terminated string inside `b[o..o + len]`.
pub fn cstr(b: &[u8], o: usize, len: usize) -> String {
    let field = &b[o..(o + len).min(b.len())];
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}
