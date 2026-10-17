//! The load checks, in the order the original makes them
//! (`docs/specs/career-save-screens-2.md` section 9.3 and the rewrite rules of section 14).
//!
//! Enforced: the file length, the header total size, the magic, the header checksum, the descriptor and profile
//! sizes, the descriptor type id and size, the default stable hash and, last, the profile MD5. **Ignored**: a wrong
//! descriptor checksum or profile checksum (the error is overwritten with success), so an editor that changes the
//! profile without fixing those two words still loads as long as the MD5 is right.

use super::state::ProfileState;
use crate::save::crc::container_crc;
use crate::save::layout::{
    DESCRIPTOR_LEN, FILE_LEN, HEADER_LEN, MAGIC, PROFILE_AT, PROFILE_LEN, PROFILE_TYPE_ID, header,
};
use crate::save::{Integrity, ProfileSave};
use crate::settings::Flags;

/// The tag `Game` of the gameplay blob (`0x656D6147`) and the least version the loader accepts.
pub const BLOB_TAG: u32 = 0x656D_6147;
/// The least blob version.
pub const BLOB_MIN_VERSION: u32 = 8;

/// The library error code of a short read or a failed length query.
pub const CODE_SHORT_READ: u8 = 7;
/// The library error code of a bad container (sizes, magic, header checksum).
pub const CODE_BAD_CONTAINER: u8 = 8;
/// The library error code of a missing file or folder (the caller reports it: this module gets bytes).
pub const CODE_MISSING: u8 = 9;

/// Why a load failed. Every failure restores the snapshot of the profile taken before the load and posts the
/// notice `MC_CORRUPT_PROFILE_XBOX` (the alias "appears to be damaged"); the library errors show "unable to load X"
/// first unless it is masked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    /// A read came up short, or the file is shorter than the container (code 7).
    #[error("the file is too short")]
    ShortRead,
    /// The header total size, the magic, the header checksum, the descriptor size or the profile size is wrong
    /// (code 8).
    #[error("the container header is not valid")]
    BadContainer,
    /// The descriptor type id is not `0x10D` or its size is not `0xF848` (game check 8).
    #[error("the descriptor does not describe a profile")]
    BadDescriptor,
    /// The default stable hash differs from the running game (a save from another car set). The settings may
    /// already have been copied when this stops the load; the screens restore the snapshot.
    #[error("the default stable hash differs")]
    StableHashMismatch,
    /// The profile MD5 differs. It is checked last, after the data was copied into the live profile.
    #[error("the profile MD5 does not match")]
    Md5Mismatch,
}

impl LoadError {
    /// The library error code for the library errors, `None` for the game checks.
    pub const fn library_code(self) -> Option<u8> {
        match self {
            LoadError::ShortRead => Some(CODE_SHORT_READ),
            LoadError::BadContainer => Some(CODE_BAD_CONTAINER),
            _ => None,
        }
    }

    /// True when the "unable to load X" prompt comes first (library errors); the game checks go straight to the
    /// damaged notice.
    pub const fn shows_unable_prompt(self) -> bool {
        self.library_code().is_some()
    }
}

/// Whether the gameplay blob was taken. A bad blob (MD5, tag `Game`, version at least 8) is **silently skipped**
/// and the old gameplay state stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobStatus {
    /// MD5, tag and version are fine.
    Accepted,
    /// The blob is skipped.
    Skipped,
}

/// A file that passed the load checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadReport {
    /// The profile file.
    pub save: ProfileSave,
    /// The checksums as found. A false descriptor or profile checksum was ignored.
    pub integrity: Integrity,
    /// What the loader does with the gameplay blob.
    pub blob: BlobStatus,
}

impl LoadReport {
    /// The flags word after the load: the stored flags are merged into the running ones and game over is cleared
    /// (`(old | stored) & !0x800`). The bonus-content probes may set bits `0x40000` and `0x20000` besides; they are
    /// not modelled.
    pub fn merged_flags(&self, running: Flags) -> Flags {
        match self.save.settings() {
            Ok(settings) => running.merged_on_load(settings.flags),
            Err(_) => running,
        }
    }

    /// The effects of a successful first-player load on the profile bytes and the autosave manager: profile loaded,
    /// "switched off by error" cleared, and, when the option is on, quiet mode armed. Returns whether it armed.
    pub fn apply_success(&self, profile: &mut ProfileState, manager: &mut super::autosave::AutosaveManager) -> bool {
        profile.load_succeeded(manager);
        if !manager.option_on {
            return false;
        }
        manager.arm_quiet_mode();
        true
    }
}

fn u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn blob_status(save: &ProfileSave) -> BlobStatus {
    let Ok(settings) = save.settings() else { return BlobStatus::Skipped };
    let tag_ok = u32_le(&settings.gameplay_blob[..], 0x10) == BLOB_TAG;
    let version_ok = u32_le(&settings.gameplay_blob[..], 0x14) >= BLOB_MIN_VERSION;
    match tag_ok && version_ok && settings.gameplay_digest_ok() {
        true => BlobStatus::Accepted,
        false => BlobStatus::Skipped,
    }
}

/// Checks a profile file the way the loader does, in order:
///
/// 1. the file holds at least the 0x1C-byte header, else code 7;
/// 2. the header total size equals the file length, the magic is `20CM` and the header checksum over bytes
///    `0..0x18` matches the word at `0x18`, else code 8;
/// 3. the descriptor size is 8 and the profile size is `0xF848`, else code 8; a file of any other length than the
///    one those sizes describe is code 7 when shorter and code 8 when longer (the spec does not describe a longer
///    file: refused);
/// 4. the descriptor checksum and the profile checksum are computed and a mismatch is **ignored**;
/// 5. the descriptor type is `0x10D` and its size `0xF848`, else the game check fails;
/// 6. the settings block is read (the blob is skipped silently when bad), then the default stable hash against
///    `expected_stable_hash`, else the load stops;
/// 7. the profile MD5, last.
pub fn check_load(bytes: &[u8], expected_stable_hash: u32) -> Result<LoadReport, LoadError> {
    if bytes.len() < HEADER_LEN {
        return Err(LoadError::ShortRead);
    }
    let crc_ok = container_crc(&bytes[..header::HEADER_CRC]) == Some(u32_le(bytes, header::HEADER_CRC));
    if u32_le(bytes, header::TOTAL_SIZE) as usize != bytes.len() || u32_le(bytes, 0) != MAGIC || !crc_ok {
        return Err(LoadError::BadContainer);
    }
    if u32_le(bytes, header::DESCRIPTOR_SIZE) as usize != DESCRIPTOR_LEN
        || u32_le(bytes, header::PROFILE_SIZE) as usize != PROFILE_LEN
    {
        return Err(LoadError::BadContainer);
    }
    if bytes.len() < FILE_LEN {
        return Err(LoadError::ShortRead);
    }
    if bytes.len() > FILE_LEN {
        return Err(LoadError::BadContainer);
    }
    if u32_le(bytes, header::TYPE_ID) != PROFILE_TYPE_ID
        || u32_le(bytes, header::DESCRIPTOR_PROFILE_SIZE) as usize != PROFILE_LEN
    {
        return Err(LoadError::BadDescriptor);
    }
    let save = ProfileSave::from_unchecked(bytes.to_vec());
    let blob = blob_status(&save);
    if save.default_stable_hash() != expected_stable_hash {
        return Err(LoadError::StableHashMismatch);
    }
    let integrity = save.integrity();
    if !integrity.profile_md5 {
        return Err(LoadError::Md5Mismatch);
    }
    debug_assert_eq!(PROFILE_AT, HEADER_LEN + DESCRIPTOR_LEN);
    Ok(LoadReport { save, integrity, blob })
}

#[cfg(test)]
mod tests;
