//! Short text the settings can hold: [`InlineText`] is `Copy` (the settings are), so it keeps its bytes inline.

use std::fmt;

/// UTF-8 text of at most `N` bytes (`N` at most 255), stored inline.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InlineText<const N: usize> {
    len: u8,
    bytes: [u8; N],
}

impl<const N: usize> InlineText<N> {
    /// The empty text.
    pub const EMPTY: Self = Self { len: 0, bytes: [0; N] };

    /// `text` trimmed, or an error when it is longer than `N` bytes or holds a control character.
    pub fn new(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.len() > N || N > usize::from(u8::MAX) {
            return Err(format!("at most {N} bytes"));
        }
        if text.chars().any(char::is_control) {
            return Err("no control characters".into());
        }
        let mut bytes = [0; N];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Ok(Self { len: text.len() as u8, bytes })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Default for InlineText<N> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl<const N: usize> fmt::Debug for InlineText<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const N: usize> fmt::Display for InlineText<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_trimmed_kept_and_bounded() {
        let text = InlineText::<8>::new("  Pietro ").unwrap();
        assert_eq!(text.as_str(), "Pietro");
        assert_eq!(format!("{text:?}"), "\"Pietro\"");
        assert!(InlineText::<4>::new("toolong").is_err());
        assert!(InlineText::<8>::new("a\tb").is_err());
        assert!(InlineText::<8>::new("").unwrap().is_empty());
        assert_eq!(InlineText::<12>::new("Ünïcode").unwrap().as_str(), "Ünïcode");
    }
}
