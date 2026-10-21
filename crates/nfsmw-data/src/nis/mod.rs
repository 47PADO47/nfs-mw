//! The NIS cutscene data of the install: the ICE camera packs of `GLOBAL/InGameB.bun` by scene name. Readers of camera
//! tracks (the pursuit breaker's moment, the NIS cutscene player, the story FMV stand-ins) share one [`CameraLibrary`].
//!
//! - [`cameras`]: the NIS and FMV camera packs, their sets (one per scene name) and their `Track <n>` tracks.

pub mod cameras;

pub use cameras::{CameraLibrary, track_numbered};
