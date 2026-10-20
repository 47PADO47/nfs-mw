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
//! - [`speech_chase`]: the thresholds the chase's speech flows read (`speechtune`, and the roadblock and 911 fields of a heat row).
//! - [`time_of_day`]: the 8 lighting presets from the attribute database and the blend between two; the day-night clock that picks the sun and the lights is in [`day_night`].
//! - [`day_night`]: the day-night clock's curves (hour, sun phase, lights on and off) and its settings.
//! - [`career_cars`]: the career car lot's cars, prices and unlock ranks from `GLOBAL/FE_ATTRIB.bin`.
//! - [`gameplay`]: the race events, their box triggers (start, finish, checkpoints) and the auto shop and safe house zones.

pub mod camera;
pub mod car;
pub mod career_cars;
pub mod cooling;
pub mod day_night;
mod files;
pub mod fog;
pub mod game;
pub mod gameplay;
pub mod minimap;
pub mod music;
pub mod pursuit;
pub mod pursuit_siren;
pub mod rank_thresholds;
pub mod shared;
pub mod sound;
pub mod speech;
pub mod speech_chase;
pub mod time_of_day;
pub mod traffic;
pub mod vehicle_effects;
pub mod visual_treatment;
pub mod world;

pub use files::read_unwrapped;
pub use files::{memo, shared_unwrapped};
