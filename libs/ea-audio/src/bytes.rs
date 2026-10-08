//! Bounds-checked integer reads from byte slices.

use crate::error::{Error, Result};

/// Byte order of the values inside a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

fn take<const N: usize>(data: &[u8], at: usize) -> Result<[u8; N]> {
    let truncated = Error::Truncated { offset: at as u64, needed: N };
    let end = at.checked_add(N).ok_or(truncated.clone())?;
    let bytes = data.get(at..end).ok_or(truncated)?;
    let mut out = [0u8; N];
    out.copy_from_slice(bytes);
    Ok(out)
}

pub fn u8_at(data: &[u8], at: usize) -> Result<u8> {
    data.get(at).copied().ok_or(Error::Truncated { offset: at as u64, needed: 1 })
}

pub fn u16_le(data: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(take(data, at)?))
}

pub fn u32_le(data: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(take(data, at)?))
}

pub fn u32_be(data: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(take(data, at)?))
}

/// A `u32` in the given byte order.
pub fn u32_in(data: &[u8], at: usize, endian: Endian) -> Result<u32> {
    match endian {
        Endian::Little => u32_le(data, at),
        Endian::Big => u32_be(data, at),
    }
}

/// The slice `data[start..start + len]`, or `Truncated`.
pub fn slice(data: &[u8], start: usize, len: usize) -> Result<&[u8]> {
    let truncated = Error::Truncated { offset: start as u64, needed: len };
    let end = start.checked_add(len).ok_or(truncated.clone())?;
    data.get(start..end).ok_or(truncated)
}
