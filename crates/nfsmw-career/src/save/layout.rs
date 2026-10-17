//! Offsets of the profile file (`docs/formats/career-save.md` sections 1 and 2). Every integer is little endian.
//!
//! "File" offsets count from the start of the file; "profile" offsets from the start of the profile buffer, which
//! begins at file offset `0x24`.

/// The magic `20CM` as a little-endian u32.
pub const MAGIC: u32 = 0x4D43_3032;
/// Length of the container header.
pub const HEADER_LEN: usize = 0x1C;
/// Length of the descriptor that follows the header.
pub const DESCRIPTOR_LEN: usize = 8;
/// File offset of the profile buffer.
pub const PROFILE_AT: usize = HEADER_LEN + DESCRIPTOR_LEN;
/// Length of the profile buffer: `23,033 + 19 x (0x30 + 0x825)`.
pub const PROFILE_LEN: usize = 0xF848;
/// Length of the whole file.
pub const FILE_LEN: usize = PROFILE_AT + PROFILE_LEN;
/// The profile type id in the descriptor.
pub const PROFILE_TYPE_ID: u32 = 0x10D;

/// File offsets of the header words.
pub mod header {
    /// Total file size.
    pub const TOTAL_SIZE: usize = 0x04;
    /// Descriptor size (8).
    pub const DESCRIPTOR_SIZE: usize = 0x08;
    /// Profile buffer size.
    pub const PROFILE_SIZE: usize = 0x0C;
    /// Checksum of the descriptor.
    pub const DESCRIPTOR_CRC: usize = 0x10;
    /// Checksum of the profile buffer.
    pub const PROFILE_CRC: usize = 0x14;
    /// Checksum of header bytes `0x00..0x18`.
    pub const HEADER_CRC: usize = 0x18;
    /// Descriptor word 0 (profile type id).
    pub const TYPE_ID: usize = 0x1C;
    /// Descriptor word 1 (profile size again).
    pub const DESCRIPTOR_PROFILE_SIZE: usize = 0x20;
}

/// Offsets and sizes inside the profile buffer (file offset minus `0x24`).
pub mod profile {
    /// The 16-byte version field (zero filled); the MD5 range starts after it.
    pub const VERSION_LEN: usize = 16;
    /// The default stable hash (`u32`, file `0x5A2D`), which depends on the installed car data.
    pub const DEFAULT_STABLE_HASH: usize = 0x5A09;
    /// The profile name, 32 bytes of zero-padded text (file `0x5A31`).
    pub const NAME: usize = 0x5A0D;
    /// Length of the profile name.
    pub const NAME_LEN: usize = 32;
    /// The car stable (file `0x5BC5`).
    pub const STABLE: usize = 0x5BA1;
    /// The "career completed at least once" byte (file `0xE88D`).
    pub const COMPLETED_ONCE: usize = 0xE869;
    /// The MD5 at the end of the buffer (file `0xF85C`).
    pub const MD5: usize = 0xF838;
    /// Length of the MD5.
    pub const MD5_LEN: usize = 16;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sizes_agree_with_the_documented_file() {
        assert_eq!(FILE_LEN, 0xF86C);
        assert_eq!(FILE_LEN, 63_596);
        assert_eq!(PROFILE_LEN, 23_033 + 19 * (0x30 + 0x825));
    }

    #[test]
    fn profile_offsets_are_the_documented_file_offsets_minus_the_prefix() {
        assert_eq!(profile::DEFAULT_STABLE_HASH + PROFILE_AT, 0x5A2D);
        assert_eq!(profile::NAME + PROFILE_AT, 0x5A31);
        assert_eq!(profile::STABLE + PROFILE_AT, 0x5BC5);
        assert_eq!(profile::COMPLETED_ONCE + PROFILE_AT, 0xE88D);
        assert_eq!(profile::MD5 + PROFILE_AT, 0xF85C);
        assert_eq!(profile::MD5 + profile::MD5_LEN, PROFILE_LEN);
        assert_eq!(profile::STABLE + crate::stable::STABLE_LEN, profile::COMPLETED_ONCE);
    }
}
