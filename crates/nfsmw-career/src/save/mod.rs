//! The profile file: container checksums, the profile MD5 and in-place access to the career blocks
//! (`docs/formats/career-save.md`).
//!
//! - [`crc`]: the container checksum.
//! - [`layout`]: offsets and sizes.
//! - [`ProfileSave`]: a whole file, with the settings and the stable parsed out and written back in place.
//!
//! Left out: the default stable hash (it depends on the installed car data, so a save written by the rewrite loads
//! in the original only with the same car set; the stored value is kept as it is), the gameplay blob contents and
//! the other blocks (options, playlist, statistics, race settings, controller bindings), which stay verbatim.

pub mod crc;
pub mod layout;
pub mod milestones_block;
mod profile;

pub use profile::{Integrity, ProfileSave, SaveError};

#[cfg(test)]
mod tests;
