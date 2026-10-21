//! EA NIS cutscene scenes of EA Black Box games: the `AnimScene` container of a bundle (its header and its character
//! records), the scene clock, and the placement of a scene in the world.
//!
//! Every reader takes bytes and never opens files; the caller supplies the bundle (for example from the game install).
//! Nothing here depends on a renderer or on a particular game. The formats are in `docs/specs/nis-scenes.md`, and the
//! timing and placement rules in `docs/specs/nis-player.md`.

mod bytes;
mod character;
mod error;
pub mod placement;
mod scene;
mod scene_type;
pub mod timeline;

#[cfg(test)]
mod tests;

pub use character::{Character, RECORD_LEN as CHARACTER_RECORD_LEN};
pub use error::{Error, Result};
pub use scene::{ANIM_SCENE, SCENE_ENTITY, SCENE_HEADER, Scene, find_scene};
pub use scene_type::SceneType;
pub use timeline::{Playback, PlaybackState, SceneClock};
