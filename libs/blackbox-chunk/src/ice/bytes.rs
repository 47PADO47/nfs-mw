//! Little-endian field reads with bounds checks. Every read returns `None` past the end.

pub(super) fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

pub(super) fn i16_at(data: &[u8], at: usize) -> Option<i16> {
    let bytes: [u8; 2] = data.get(at..at.checked_add(2)?)?.try_into().ok()?;
    Some(i16::from_le_bytes(bytes))
}

pub(super) fn f32_at(data: &[u8], at: usize) -> Option<f32> {
    let bytes: [u8; 4] = data.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(f32::from_le_bytes(bytes))
}
