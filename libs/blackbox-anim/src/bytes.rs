//! Bounds-checked little-endian reads from a byte slice. A read past the end returns `Error::Truncated`
//! and never panics.

use crate::{Error, Result};

/// The `len` bytes at `offset`.
pub(crate) fn slice<'a>(data: &'a [u8], offset: usize, len: usize, what: &'static str) -> Result<&'a [u8]> {
    let end = offset.checked_add(len).filter(|&end| end <= data.len()).ok_or(Error::Truncated { what, offset })?;
    Ok(&data[offset..end])
}

fn array<const N: usize>(data: &[u8], offset: usize, what: &'static str) -> Result<[u8; N]> {
    let bytes = slice(data, offset, N, what)?;
    let mut out = [0u8; N];
    out.copy_from_slice(bytes);
    Ok(out)
}

pub(crate) fn u8_at(data: &[u8], offset: usize, what: &'static str) -> Result<u8> {
    Ok(slice(data, offset, 1, what)?[0])
}

pub(crate) fn u16_at(data: &[u8], offset: usize, what: &'static str) -> Result<u16> {
    Ok(u16::from_le_bytes(array(data, offset, what)?))
}

pub(crate) fn u32_at(data: &[u8], offset: usize, what: &'static str) -> Result<u32> {
    Ok(u32::from_le_bytes(array(data, offset, what)?))
}

pub(crate) fn f32_at(data: &[u8], offset: usize, what: &'static str) -> Result<f32> {
    Ok(f32::from_bits(u32_at(data, offset, what)?))
}

/// The NUL-terminated string at `offset`, without the NUL. The string must be ASCII.
pub(crate) fn cstr_at<'a>(data: &'a [u8], offset: usize, what: &'static str) -> Result<&'a str> {
    let rest = data.get(offset..).ok_or(Error::Truncated { what, offset })?;
    let nul = rest.iter().position(|&byte| byte == 0).ok_or(Error::Truncated { what, offset: data.len() })?;
    let text = &rest[..nul];
    if !text.is_ascii() {
        return Err(Error::Invalid(format!("{what} at offset {offset:#x} is not ASCII")));
    }
    // ASCII is valid UTF-8, so this cannot fail; the error arm only keeps the function free of unwraps.
    std::str::from_utf8(text).map_err(|_| Error::Invalid(format!("{what} at offset {offset:#x} is not UTF-8")))
}
