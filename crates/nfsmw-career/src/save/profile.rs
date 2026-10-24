//! A whole profile file with the career-relevant blocks readable and writable in place
//! (`docs/formats/career-save.md`).
//!
//! [`ProfileSave`] keeps the file bytes. The career blocks (settings, stable) are parsed out on demand and
//! written back over the same bytes, so everything this crate does not understand (the options, the playlist,
//! the statistics, the controller bindings, the unknown bytes inside the blocks) stays verbatim. When the bytes
//! are turned back into a file the profile MD5 and the three container checksums are recomputed, so an
//! unchanged, consistent save comes back byte for byte.

use super::crc::container_crc;
use super::layout::{DESCRIPTOR_LEN, FILE_LEN, HEADER_LEN, MAGIC, PROFILE_AT, PROFILE_LEN, PROFILE_TYPE_ID};
use super::layout::{header, profile};
use crate::LayoutError;
use crate::bytes::{put_u32, u32_at};
use crate::rapsheet::PursuitHistory;
use crate::settings::CareerSettings;
use crate::stable::{STABLE_LEN, Stable};
use save_store::digest::md5;

/// Why a profile file was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SaveError {
    /// The file does not have the exact length of a profile file.
    #[error("a profile file is {FILE_LEN} bytes but this one has {0}")]
    BadLength(usize),
    /// The first four bytes are not `20CM`.
    #[error("bad magic {0:#010x}")]
    BadMagic(u32),
    /// The stored total, descriptor or profile size, or the profile type id, does not match.
    #[error("the container header does not describe a profile ({0})")]
    BadHeader(&'static str),
    /// The checksum of header bytes `0..0x18` does not match; the original refuses such a file.
    #[error("the header checksum does not match")]
    HeaderChecksum,
    /// The profile MD5 does not match; the original refuses such a file.
    #[error("the profile MD5 does not match")]
    Md5Mismatch,
    /// A block was shorter than its layout (cannot happen for a parsed file).
    #[error(transparent)]
    Layout(#[from] LayoutError),
}

/// Which integrity values of a file are consistent. The original rejects a file whose header checksum or MD5 is
/// wrong; the other two are computed and compared but a mismatch does not by itself reject the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Integrity {
    /// The descriptor checksum at `0x10`.
    pub descriptor_crc: bool,
    /// The profile-buffer checksum at `0x14`.
    pub profile_crc: bool,
    /// The header checksum at `0x18`.
    pub header_crc: bool,
    /// The MD5 at the end of the profile buffer.
    pub profile_md5: bool,
}

impl Integrity {
    /// True when every value matches.
    pub const fn all_ok(&self) -> bool {
        self.descriptor_crc && self.profile_crc && self.header_crc && self.profile_md5
    }
}

/// A profile file, 63,596 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSave {
    bytes: Vec<u8>,
}

impl ProfileSave {
    /// Reads a profile file. Checks what the original loader checks: the length, the magic, the sizes and type
    /// id of the container, the header checksum and the profile MD5. A wrong descriptor or profile checksum is
    /// accepted (see [`Integrity`]) and fixed when the file is written again.
    pub fn parse(file: &[u8]) -> Result<ProfileSave, SaveError> {
        if file.len() != FILE_LEN {
            return Err(SaveError::BadLength(file.len()));
        }
        let magic = u32_at(file, 0);
        if magic != MAGIC {
            return Err(SaveError::BadMagic(magic));
        }
        let sizes_ok = [
            (header::TOTAL_SIZE, FILE_LEN, "total size"),
            (header::DESCRIPTOR_SIZE, DESCRIPTOR_LEN, "descriptor size"),
            (header::PROFILE_SIZE, PROFILE_LEN, "profile size"),
            (header::DESCRIPTOR_PROFILE_SIZE, PROFILE_LEN, "descriptor profile size"),
        ];
        for (at, expected, what) in sizes_ok {
            if u32_at(file, at) as usize != expected {
                return Err(SaveError::BadHeader(what));
            }
        }
        if u32_at(file, header::TYPE_ID) != PROFILE_TYPE_ID {
            return Err(SaveError::BadHeader("profile type id"));
        }
        let save = ProfileSave { bytes: file.to_vec() };
        let integrity = save.integrity();
        if !integrity.header_crc {
            return Err(SaveError::HeaderChecksum);
        }
        if !integrity.profile_md5 {
            return Err(SaveError::Md5Mismatch);
        }
        Ok(save)
    }

    /// Wraps a profile buffer of [`PROFILE_LEN`] bytes in a fresh container: header, descriptor, MD5 and all
    /// three checksums are computed.
    pub fn from_profile_buffer(buffer: &[u8]) -> Result<ProfileSave, SaveError> {
        if buffer.len() != PROFILE_LEN {
            return Err(SaveError::Layout(LayoutError {
                what: "profile buffer",
                need: PROFILE_LEN,
                got: buffer.len(),
            }));
        }
        Ok(ProfileSave::wrap(buffer))
    }

    /// A blank profile for a new career named `alias` (`docs/formats/career-save.md` sections 1 and 2). The settings
    /// are [`CareerSettings::new_career`] (stage 16, cash 0, career-started flag), the stable is [`Stable::empty`], the
    /// statistics block is zero and the name is `alias` (cut to 32 bytes).
    ///
    /// Left as zero, on purpose: the gameplay blob (so the milestone block has no records yet and the save's header
    /// counts are 0), the default stable hash at `0x5A2D` (it depends on the installed car data; the caller must set
    /// it before the original can load the file), and the serial and the SMS labels (see [`CareerSettings::new_career`]).
    pub fn new_blank(alias: &str) -> Result<ProfileSave, SaveError> {
        let mut buffer = vec![0u8; PROFILE_LEN];
        CareerSettings::new_career().write_into_profile(&mut buffer)?;
        let stable = Stable::empty().to_bytes();
        buffer[profile::STABLE..profile::STABLE + STABLE_LEN].copy_from_slice(&stable);
        let name = alias.as_bytes();
        let n = name.len().min(profile::NAME_LEN);
        buffer[profile::NAME..profile::NAME + n].copy_from_slice(&name[..n]);
        Ok(ProfileSave::wrap(&buffer))
    }

    /// Wraps a profile buffer of [`PROFILE_LEN`] bytes (the caller checked the length) in a fresh, sealed container.
    fn wrap(buffer: &[u8]) -> ProfileSave {
        let mut bytes = vec![0u8; FILE_LEN];
        put_u32(&mut bytes, 0, MAGIC);
        put_u32(&mut bytes, header::TOTAL_SIZE, FILE_LEN as u32);
        put_u32(&mut bytes, header::DESCRIPTOR_SIZE, DESCRIPTOR_LEN as u32);
        put_u32(&mut bytes, header::PROFILE_SIZE, PROFILE_LEN as u32);
        put_u32(&mut bytes, header::TYPE_ID, PROFILE_TYPE_ID);
        put_u32(&mut bytes, header::DESCRIPTOR_PROFILE_SIZE, PROFILE_LEN as u32);
        bytes[PROFILE_AT..].copy_from_slice(buffer);
        let mut save = ProfileSave { bytes };
        save.seal();
        save
    }

    /// Wraps file bytes whose length is already known to be right, without any other check. The caller (the load
    /// path) does its own checks in the order of the original.
    pub(crate) fn from_unchecked(bytes: Vec<u8>) -> ProfileSave {
        ProfileSave { bytes }
    }

    /// The file bytes as they are now (checksums as loaded, not recomputed).
    pub fn raw(&self) -> &[u8] {
        &self.bytes
    }

    /// The profile buffer part of the file.
    pub fn profile_buffer(&self) -> &[u8] {
        &self.bytes[PROFILE_AT..]
    }

    /// Checks all four integrity values against the bytes.
    pub fn integrity(&self) -> Integrity {
        let b = &self.bytes;
        let crc_at = |at: usize, range: std::ops::Range<usize>| container_crc(&b[range]) == Some(u32_at(b, at));
        Integrity {
            descriptor_crc: crc_at(header::DESCRIPTOR_CRC, HEADER_LEN..PROFILE_AT),
            profile_crc: crc_at(header::PROFILE_CRC, PROFILE_AT..FILE_LEN),
            header_crc: crc_at(header::HEADER_CRC, 0..header::HEADER_CRC),
            profile_md5: self.profile_md5() == b[self.md5_range()],
        }
    }

    /// The profile MD5 as it should be: over the profile buffer after its 16-byte version field, up to the digest.
    fn profile_md5(&self) -> [u8; 16] {
        md5(&self.bytes[PROFILE_AT + profile::VERSION_LEN..PROFILE_AT + profile::MD5])
    }

    fn md5_range(&self) -> std::ops::Range<usize> {
        PROFILE_AT + profile::MD5..FILE_LEN
    }

    /// Recomputes the profile MD5 and the three container checksums in place.
    fn seal(&mut self) {
        let digest = self.profile_md5();
        let range = self.md5_range();
        self.bytes[range].copy_from_slice(&digest);
        let descriptor = container_crc(&self.bytes[HEADER_LEN..PROFILE_AT]).unwrap_or(0);
        let profile = container_crc(&self.bytes[PROFILE_AT..]).unwrap_or(0);
        put_u32(&mut self.bytes, header::DESCRIPTOR_CRC, descriptor);
        put_u32(&mut self.bytes, header::PROFILE_CRC, profile);
        let header = container_crc(&self.bytes[..header::HEADER_CRC]).unwrap_or(0);
        put_u32(&mut self.bytes, header::HEADER_CRC, header);
    }

    /// The file with the profile MD5 and the three checksums recomputed. A consistent, unchanged save comes back
    /// byte for byte.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut sealed = self.clone();
        sealed.seal();
        sealed.bytes
    }

    /// The career settings.
    pub fn settings(&self) -> Result<CareerSettings, LayoutError> {
        CareerSettings::from_profile_bytes(self.profile_buffer())
    }

    /// Writes the settings over their bytes; nothing else changes.
    pub fn set_settings(&mut self, settings: &CareerSettings) -> Result<(), LayoutError> {
        settings.write_into_profile(&mut self.bytes[PROFILE_AT..])
    }

    /// The car stable.
    pub fn stable(&self) -> Result<Stable, LayoutError> {
        Stable::parse(&self.profile_buffer()[profile::STABLE..])
    }

    /// Writes the stable over its bytes; nothing else changes.
    pub fn set_stable(&mut self, stable: &Stable) {
        let at = PROFILE_AT + profile::STABLE;
        self.bytes[at..at + STABLE_LEN].copy_from_slice(&stable.to_bytes());
    }

    /// The pursuit history (the statistics block at file `0xE88E`).
    pub fn history(&self) -> Result<PursuitHistory, LayoutError> {
        PursuitHistory::from_block(&self.profile_buffer()[profile::HISTORY..])
    }

    /// Writes the typed fields of the history over their bytes; the rest of the block is untouched.
    pub fn set_history(&mut self, history: &PursuitHistory) -> Result<(), LayoutError> {
        history.write_into(&mut self.bytes[PROFILE_AT + profile::HISTORY..])
    }

    /// The default stable hash stored at file `0x5A2D`. It depends on the installed car data and is never
    /// recomputed here: the original refuses a save whose value differs from the running game.
    pub fn default_stable_hash(&self) -> u32 {
        u32_at(self.profile_buffer(), profile::DEFAULT_STABLE_HASH)
    }

    /// The profile name (the alias): the 32 name bytes up to the first zero (lossy for non-UTF-8 bytes).
    pub fn alias(&self) -> String {
        let name = &self.profile_buffer()[profile::NAME..profile::NAME + profile::NAME_LEN];
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        String::from_utf8_lossy(&name[..end]).into_owned()
    }

    /// The "career completed at least once" byte, which unlocks every car in the non-career modes.
    pub fn career_completed_once(&self) -> bool {
        self.profile_buffer()[profile::COMPLETED_ONCE] != 0
    }

    /// Mark this profile as having completed a career. Writing refreshes its checksums normally.
    pub fn mark_career_completed(&mut self) {
        self.bytes[PROFILE_AT + profile::COMPLETED_ONCE] = 1;
    }
}
