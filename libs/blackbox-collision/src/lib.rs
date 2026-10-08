//! Static world collision and collision bounds of EA Black Box games (Need for Speed: Most Wanted).
//! Spec: `docs/formats/collision.md`.
//!
//! - [`CollisionPack`]: one streaming section's collision (`0x3B801`): instances placing
//!   [`Article`]s, which hold triangle [`Strip`]s, [`Barrier`]s and a surface table.
//! - [`Grid`]: the world-wide xz grid (`0x3B800`) that says which instances a place can touch.
//! - [`BoundsSet`]: the collision boxes and spheres of a car or prop (`0x8003B900`).
//! - [`CollisionWorld`]: loaded packs plus the grid, with a segment query
//!   ([`CollisionWorld::ray_cast`]).
//!
//! Coordinates are the game's physics space: x right, y up, z forward, metres. This crate never
//! touches the filesystem: it takes bytes.

mod article;
mod bounds;
mod bytes;
pub mod carp;
mod error;
mod grid;
mod instance;
mod math;
mod object;
mod pack;
mod query;
mod world;

#[cfg(test)]
mod tests;

pub use article::{
    Article, BARRIER_TWO_SIDED, Barrier, PackedVert, RADIUS_SCALE, STRIP_FACING_UNKNOWN, STRIP_UP_FACING,
    SURFACE_NO_GROUND, Strip, Triangle, VERT_SCALE,
};
pub use bounds::{Bounds, BoundsSet, Shape, find_bounds, flags as bounds_flags, read_bounds_sets};
pub use error::{Error, Result};
pub use grid::{Grid, GridNode, InstanceRef};
pub use instance::{FLAG_DYNAMIC, FLAG_Y_NOT_UP, Instance};
pub use math::Vec3;
pub use object::Object;
pub use pack::{CollisionPack, GROUP_EXCLUSION, read_collision_packs};
pub use query::{Hit, HitKind, RayOptions};
pub use world::CollisionWorld;
