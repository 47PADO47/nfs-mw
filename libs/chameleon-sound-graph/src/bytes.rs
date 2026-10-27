//! Little-endian reads that return `None` past the end of a block.

pub(crate) fn u32_at(block: &[u8], at: usize) -> Option<u32> {
    let bytes = block.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub(crate) fn u16_at(block: &[u8], at: usize) -> Option<u16> {
    let bytes = block.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

pub(crate) fn f32_at(block: &[u8], at: usize) -> Option<f32> {
    u32_at(block, at).map(f32::from_bits)
}

/// The string of a `StringBase` field at `at`: an offset relative to the field and a length that counts the zero.
pub(crate) fn string_at(block: &[u8], at: usize) -> Option<String> {
    let offset = u32_at(block, at)? as usize;
    let length = u32_at(block, at + 4)? as usize;
    if length == 0 {
        return Some(String::new());
    }
    let start = at.checked_add(offset)?;
    let bytes = block.get(start..start.checked_add(length)?)?;
    let end = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
}
