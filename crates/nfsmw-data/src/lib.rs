//! NFS: Most Wanted (2005, PC) data assembly on top of the engine-generic
//! `libs/` crates: which files the game uses and how they fit together.
//! Renderer-free, so it can be tested and reused by tools.
//!
//! - [`game`]: the install spec (registry key, required files, known builds);
//! - [`car`]: one car's solids and textures;
//! - [`minimap`]: the map tiles and the calibration of the open city's minimap;
//! - [`world`]: the streamed city (`TRACKS/L2RA.BUN` + `STREAML2RA.BUN`).
//! - [`pursuit`]: the cop waves of each heat level;
//! - [`traffic`]: the car list and speeds of each traffic pattern;
//! - [`music`]: the licensed songs of the radio, from the attribute database;
//! - [`sound`]: a car's engine sound set (`.gin` loops, banks and mix tuning) from the attribute database;
//! - [`speech`]: the scheduling data of the police dispatch's speech events from the attribute database.
//! - [`time_of_day`]: the 8 lighting presets from the attribute database and the blend between two (no clock yet).

pub mod car;
mod files;
pub mod fog;
pub mod game;
pub mod minimap;
pub mod music;
pub mod pursuit;
pub mod sound;
pub mod speech;
pub mod time_of_day;
pub mod traffic;
pub mod vehicle_effects;
pub mod visual_treatment;
pub mod world;

pub use files::read_unwrapped;
