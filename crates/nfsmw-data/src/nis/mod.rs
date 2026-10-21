//! The NIS cutscene data of the install: the ICE camera packs of `GLOBAL/InGameB.bun` by scene name. Readers of camera
//! tracks (the pursuit breaker's moment, the NIS cutscene player, the story FMV stand-ins) share one [`CameraLibrary`].
//!
//! - [`cameras`]: the NIS and FMV camera packs, their sets (one per scene name) and their `Track <n>` tracks.
//! - [`bundle`]: a NIS scene's bundle (`NIS/Scene_<name>_BundleB.bun`): its scene chunk and its EAGL objects.

pub mod bundle;
pub mod cameras;

pub use bundle::NisBundle;
pub use cameras::{CameraLibrary, track_numbered};
