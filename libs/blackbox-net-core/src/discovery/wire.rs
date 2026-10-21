//! Field readers for decoding. Every read goes through `get`, so no input byte sequence can make them panic.

use std::ops::Range;

use super::DiscoveryError;

/// The bytes at `range`, or `Length` when the datagram is too short for it.
pub(super) fn field(bytes: &[u8], range: Range<usize>) -> Result<&[u8], DiscoveryError> {
    bytes.get(range).ok_or(DiscoveryError::Length)
}

/// The `N` bytes starting at `at`.
pub(super) fn array<const N: usize>(bytes: &[u8], at: usize) -> Result<[u8; N], DiscoveryError> {
    let raw = field(bytes, at..at + N)?;
    raw.try_into().map_err(|_| DiscoveryError::Length)
}

/// The byte at `at`.
pub(super) fn byte(bytes: &[u8], at: usize) -> Result<u8, DiscoveryError> {
    bytes.get(at).copied().ok_or(DiscoveryError::Length)
}

/// The little-endian `u16` at `at`.
pub(super) fn le_u16(bytes: &[u8], at: usize) -> Result<u16, DiscoveryError> {
    Ok(u16::from_le_bytes(array(bytes, at)?))
}

/// The little-endian `u64` at `at`.
pub(super) fn le_u64(bytes: &[u8], at: usize) -> Result<u64, DiscoveryError> {
    Ok(u64::from_le_bytes(array(bytes, at)?))
}

/// True when every byte is zero.
pub(super) fn is_zero(bytes: &[u8]) -> bool {
    bytes.iter().all(|b| *b == 0)
}
