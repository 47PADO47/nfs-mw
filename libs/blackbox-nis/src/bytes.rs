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

pub(crate) fn u32_at(data: &[u8], offset: usize, what: &'static str) -> Result<u32> {
    Ok(u32::from_le_bytes(array(data, offset, what)?))
}

pub(crate) fn i32_at(data: &[u8], offset: usize, what: &'static str) -> Result<i32> {
    Ok(i32::from_le_bytes(array(data, offset, what)?))
}

pub(crate) fn f32_at(data: &[u8], offset: usize, what: &'static str) -> Result<f32> {
    Ok(f32::from_bits(u32_at(data, offset, what)?))
}

/// The text of a field of `len` bytes at `offset`, read up to its first NUL (or all `len` bytes when there is
/// none). The text must be ASCII.
pub(crate) fn fixed_text(data: &[u8], offset: usize, len: usize, what: &'static str) -> Result<String> {
    let field = slice(data, offset, len, what)?;
    let end = field.iter().position(|&byte| byte == 0).unwrap_or(len);
    ascii(&field[..end], offset, what)
}

/// The NUL-terminated text at `offset`, without the NUL. The NUL must be present and the text must be ASCII.
pub(crate) fn cstr_at(data: &[u8], offset: usize, what: &'static str) -> Result<String> {
    let rest = data.get(offset..).ok_or(Error::Truncated { what, offset })?;
    let nul = rest
        .iter()
        .position(|&byte| byte == 0)
        .ok_or_else(|| Error::Invalid(format!("{what} at offset {offset:#x} is not NUL-terminated")))?;
    ascii(&rest[..nul], offset, what)
}

fn ascii(bytes: &[u8], offset: usize, what: &'static str) -> Result<String> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| Error::Invalid(format!("{what} at offset {offset:#x} is not text")))?;
    if !text.is_ascii() {
        return Err(Error::Invalid(format!("{what} at offset {offset:#x} is not ASCII")));
    }
    Ok(text.to_owned())
}
