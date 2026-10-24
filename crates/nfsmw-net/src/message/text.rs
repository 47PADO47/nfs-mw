//! Bounded, filtered text (`security-gameplay.md` section 1.1, rule 7).
//!
//! [`Text::new`] normalises to NFKC, trims leading and trailing whitespace and refuses control characters, Unicode
//! bidirectional controls and a length outside `MIN..=MAX` characters. The decoder accepts only text that `new` would
//! return unchanged, so every decoded string is already clean and in one form (a look-alike written in a compatibility
//! form, such as a full-width letter, is refused on the wire). The length counts `char`s, not grapheme clusters: that
//! is the stricter bound.

use std::fmt;
use std::ops::Deref;

use unicode_normalization::{UnicodeNormalization, is_nfkc};

use super::codec::{DecodeError, LimitError, Reader, Writer};
use super::wire::Wire;

/// A player's display name: 1 to 24 characters.
pub type Name = Text<1, 24>;
/// A chat line: 1 to 200 characters.
pub type ChatText = Text<1, 200>;
/// A server's name: 1 to 32 characters.
pub type ServerName = Text<1, 32>;
/// A licence plate: 0 to 8 characters.
pub type Plate = Text<0, 8>;
/// A mod version string (semver): 1 to 32 characters.
pub type VersionText = Text<1, 32>;

/// UTF-8 text of `MIN..=MAX` characters with no control characters, no bidirectional controls and no leading or
/// trailing whitespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Text<const MIN: usize, const MAX: usize>(String);

impl<const MIN: usize, const MAX: usize> Text<MIN, MAX> {
    /// `text` in NFKC and trimmed, or [`LimitError`] when it holds a forbidden character or its length is outside
    /// `MIN..=MAX`. The text is bounded before it is normalised, as normalising can lengthen it a lot.
    pub fn new(text: &str) -> Result<Self, LimitError> {
        if text.chars().count() > MAX.saturating_mul(4) {
            return Err(LimitError);
        }
        let normal: String = text.nfkc().collect();
        let trimmed = normal.trim();
        if !is_clean(trimmed, MIN, MAX) {
            return Err(LimitError);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<const MIN: usize, const MAX: usize> Deref for Text<MIN, MAX> {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl<const MIN: usize, const MAX: usize> fmt::Display for Text<MIN, MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<const MIN: usize, const MAX: usize> Wire for Text<MIN, MAX> {
    fn put(&self, w: &mut Writer<'_>) {
        w.varint(self.0.len() as u64);
        w.bytes(self.0.as_bytes());
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let len = r.count(MAX.saturating_mul(4)).map_err(|_| DecodeError::BadText)?;
        let bytes = r.take(len)?;
        let text = std::str::from_utf8(bytes).map_err(|_| DecodeError::BadText)?;
        if text.trim() != text || !is_nfkc(text) || !is_clean(text, MIN, MAX) {
            return Err(DecodeError::BadText);
        }
        Ok(Self(text.to_owned()))
    }
}

/// A room password as both ends feed it to the key exchange: in NFKC, so the same password typed on two keyboards (or
/// with a full-width letter) is the same secret (`security.md` section 4). It is not trimmed: a space is a character of
/// a password.
pub fn password_secret(password: &str) -> Vec<u8> {
    password.nfkc().collect::<String>().into_bytes()
}

/// Whether `text` has `min..=max` characters and none of them is forbidden.
fn is_clean(text: &str, min: usize, max: usize) -> bool {
    let count = text.chars().count();
    if count < min || count > max {
        return false;
    }
    !text.chars().any(is_forbidden)
}

/// Control characters and the Unicode bidirectional controls (embeddings, overrides, isolates and marks).
fn is_forbidden(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{200E}' | '\u{200F}' | '\u{061C}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}
