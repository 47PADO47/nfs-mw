//! Content a mod declares in files rather than code (ADR 0005 section 5): a manifest per car in `cars/`, the `[music]`
//! table of `mod.toml` for the `music/` folder, a manifest per scenery pack in `scenery/`, and a manifest per interior in
//! `interiors/`.
//!
//! This module checks the schema only. Whether a `clone` names a stock car, whether a section number is free, and whether
//! the mod's files exist, is for the game to decide, because only the game knows its car tables and its index.

mod car;
mod error;
mod interior;
#[cfg(test)]
mod interior_tests;
mod music;
mod scenery;
#[cfg(test)]
mod scenery_tests;
#[cfg(test)]
mod tests;

pub use car::{CarManifest, Role, parse as parse_car};
pub use error::ContentError;
pub use interior::{Exit, InteriorManifest, Pose, Shape, Suspend, TimeOverride, parse as parse_interior};
pub use music::{MusicSection, parse as parse_music};
pub use scenery::{Collision, MAX_COMPONENT, Orientation, Place, SceneryManifest, parse as parse_scenery};

/// The folder inside a mod that holds one car manifest per car.
pub const CARS_FOLDER: &str = "cars";
/// The folder inside a mod that holds its music.
pub const MUSIC_FOLDER: &str = "music";
/// The folder inside a mod that holds one scenery manifest per pack.
pub const SCENERY_FOLDER: &str = "scenery";
/// The folder inside a mod that holds one interior manifest per interior.
pub const INTERIORS_FOLDER: &str = "interiors";

/// Longest car id or clone id, in bytes.
const MAX_ID_BYTES: usize = 32;

/// A car id or a clone id: `A-Z a-z 0-9 _ -`, 1 to 32 bytes.
fn is_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_ID_BYTES
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
