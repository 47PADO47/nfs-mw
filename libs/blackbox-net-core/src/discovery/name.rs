//! The 24-byte name field of a reply: UTF-8, cut to whole characters, zero padded.

use super::DiscoveryError;

/// Bytes in the name field.
pub(super) const NAME_LEN: usize = 24;

/// The longest prefix of `name` that fits in [`NAME_LEN`] bytes and ends on a character boundary.
fn cut(name: &str) -> &str {
    let mut end = name.len().min(NAME_LEN);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name.get(..end).unwrap_or_default()
}

/// The name field for `name`: the cut text, then zero bytes.
pub(super) fn encode(name: &str) -> [u8; NAME_LEN] {
    let mut out = [0u8; NAME_LEN];
    let kept = cut(name);
    out[..kept.len()].copy_from_slice(kept.as_bytes());
    out
}

/// Reads the name field: the text before the first zero must be UTF-8 (`Name`), and every byte after that zero must be
/// zero (`Padding`).
pub(super) fn decode(field: &[u8]) -> Result<String, DiscoveryError> {
    let len = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    let text = field.get(..len).ok_or(DiscoveryError::Name)?;
    let padding = field.get(len..).ok_or(DiscoveryError::Padding)?;
    let name = std::str::from_utf8(text).map_err(|_| DiscoveryError::Name)?;
    if padding.iter().any(|b| *b != 0) {
        return Err(DiscoveryError::Padding);
    }
    Ok(name.to_owned())
}
