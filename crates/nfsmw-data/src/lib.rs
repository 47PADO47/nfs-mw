//! NFS: Most Wanted (2005, PC) data assembly on top of the engine-generic
//! `libs/` crates: which files the game uses and how they fit together.
//! Renderer-free, so it can be tested and reused by tools.
//!
//! - [`game`]: the install spec (registry key, required files, known builds);
//! - [`car`]: one car's solids and textures;
//! - [`world`]: the streamed city (`TRACKS/L2RA.BUN` + `STREAML2RA.BUN`).
//! - [`music`]: the licensed songs of the radio, from the attribute database;
//! - [`sound`]: a car's engine sound set (`.gin` loops, banks and mix tuning) from the attribute database.

pub mod car;
mod files;
pub mod game;
pub mod music;
pub mod sound;
pub mod world;

pub use files::read_unwrapped;
