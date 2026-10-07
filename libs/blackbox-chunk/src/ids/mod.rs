//! Chunk ids, grouped by domain.
//!
//! Names follow `tools/bchunk_names.py` (the full table, sourced from the CC0
//! `dbalatoni13/nfsmw` decompilation's chunk list). Only ids the Rust code reads
//! are listed. They were checked on NFS: Most Wanted; most are shared by the
//! other EA Black Box games.

mod car;
mod geometry;
mod texture;
mod world;

pub use car::*;
pub use geometry::*;
pub use texture::*;
pub use world::*;
