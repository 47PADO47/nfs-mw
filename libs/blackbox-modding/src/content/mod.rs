//! Content a mod declares in files rather than code (ADR 0005 section 5): a manifest per car in `cars/`, and the
//! `[music]` table of `mod.toml` for the `music/` folder.
//!
//! This module checks the schema only. Whether a `clone` names a stock car, and whether the mod's files exist, is for
//! the game to decide, because only the game knows its car tables.

mod car;
mod error;
mod music;
#[cfg(test)]
mod tests;

pub use car::{CarManifest, Role, parse as parse_car};
pub use error::ContentError;
pub use music::{MusicSection, parse as parse_music};

/// The folder inside a mod that holds one car manifest per car.
pub const CARS_FOLDER: &str = "cars";
/// The folder inside a mod that holds its music.
pub const MUSIC_FOLDER: &str = "music";

/// Longest car id or clone id, in bytes.
const MAX_ID_BYTES: usize = 32;

/// A car id or a clone id: `A-Z a-z 0-9 _ -`, 1 to 32 bytes.
fn is_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_ID_BYTES
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
