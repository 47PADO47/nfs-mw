//! The header of an alias save: the magic `"Game"` and the version word that must follow it. The file starts with the
//! MD5 digest (checked by `save_store::digest`); this module reads the body after it (docs/specs/save-alias-manager.md,
//! section 8a).

// The save path that reads these is not wired yet (see docs/specs/save-alias-manager.md).
#![allow(dead_code)]

/// The magic at the start of the body: `"Game"` as a little-endian dword.
pub const MAGIC: u32 = 0x656d_6147;
/// The version word must be greater than this (the original loader refuses a smaller one).
pub const MIN_VERSION_EXCLUSIVE: u32 = 7;

#[derive(Debug, PartialEq, Eq)]
pub enum HeaderError {
    TooShort,
    BadMagic,
    OldVersion(u32),
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "the save is shorter than its header"),
            Self::BadMagic => write!(f, "the save does not start with the \"Game\" magic"),
            Self::OldVersion(v) => write!(f, "the save's version {v} is too old"),
        }
    }
}

/// The version word of a body whose magic and version check out.
pub fn check_header(body: &[u8]) -> Result<u32, HeaderError> {
    let word = |at: usize| -> Result<u32, HeaderError> {
        let bytes = body.get(at..at + 4).ok_or(HeaderError::TooShort)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    if word(0)? != MAGIC {
        return Err(HeaderError::BadMagic);
    }
    let version = word(4)?;
    if version <= MIN_VERSION_EXCLUSIVE {
        return Err(HeaderError::OldVersion(version));
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(magic: u32, version: u32) -> Vec<u8> {
        let mut b = magic.to_le_bytes().to_vec();
        b.extend_from_slice(&version.to_le_bytes());
        b
    }

    #[test]
    fn the_magic_and_a_new_enough_version_pass() {
        assert_eq!(check_header(&body(MAGIC, 8)), Ok(8));
    }

    #[test]
    fn the_wrong_magic_is_refused() {
        assert_eq!(check_header(&body(0x1234_5678, 8)), Err(HeaderError::BadMagic));
    }

    #[test]
    fn a_version_of_seven_or_less_is_refused() {
        assert_eq!(check_header(&body(MAGIC, 7)), Err(HeaderError::OldVersion(7)));
    }

    #[test]
    fn a_short_body_is_refused() {
        let mut short = MAGIC.to_le_bytes().to_vec();
        short.extend_from_slice(&[0, 0]);
        assert_eq!(check_header(&short), Err(HeaderError::TooShort));
    }
}
