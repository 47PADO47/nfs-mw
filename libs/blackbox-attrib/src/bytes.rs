//! Bounds-checked little-endian reads. `None` means "out of bounds"; callers add context.

pub(crate) fn slice(data: &[u8], offset: usize, len: usize) -> Option<&[u8]> {
    data.get(offset..offset.checked_add(len)?)
}

pub(crate) fn array<const N: usize>(data: &[u8], offset: usize) -> Option<[u8; N]> {
    slice(data, offset, N)?.try_into().ok()
}

pub(crate) fn u16_at(data: &[u8], offset: usize) -> Option<u16> {
    array(data, offset).map(u16::from_le_bytes)
}

pub(crate) fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    array(data, offset).map(u32::from_le_bytes)
}

/// The bytes from `offset` up to (not including) the next NUL.
pub(crate) fn cstr_at(data: &[u8], offset: usize) -> Option<&[u8]> {
    let rest = data.get(offset..)?;
    rest.iter().position(|&b| b == 0).map(|end| &rest[..end])
}

/// NUL-terminated strings packed back to back (empty strings included, trailing padding dropped).
pub(crate) fn cstr_list(data: &[u8]) -> impl Iterator<Item = &[u8]> {
    let end = data.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    data[..end].split(|&b| b == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads() {
        let d = [1, 2, 3, 4, 5, 0, b'a', b'b', 0, 0];
        assert_eq!(u32_at(&d, 0), Some(0x0403_0201));
        assert_eq!(u32_at(&d, 7), None);
        assert_eq!(u32_at(&d, usize::MAX), None);
        assert_eq!(cstr_at(&d, 6), Some(&b"ab"[..]));
        assert_eq!(cstr_list(&d[6..]).collect::<Vec<_>>(), [&b"ab"[..]]);
    }
}
