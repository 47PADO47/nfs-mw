//! The tracks of the user's own RIDE 6 install as second worlds of NFS: Most Wanted (2005).
//!
//! The exporter (`tools/ride6-tracks`, outside the game) reads the install and writes a track pack (`docs/formats/ride6-track-pack.md`):
//! the ground triangles by material, the pictures, the surface under each triangle and the lap. This mod reads the pack
//! ([`pack`]), cuts the ground into cells ([`cells`]), and writes each cell as a section file of the game: solids ([`model`]),
//! textures ([`textures`]) and collision with a surface per triangle ([`collision`], [`section`]), with the manifests that make
//! the cells a world ([`manifest`], [`import`]). The native entry ([`native`], [`job`], [`stamp`], [`config`]) does it in the
//! background and the game offers the world (`docs/specs/second-world.md`).

pub mod cells;
pub mod check;
pub mod collision;
pub mod config;
pub mod far;
pub mod import;
pub mod job;
pub mod lap;
pub mod manifest;
pub mod model;
mod native;
pub mod pack;
pub mod section;
pub mod stamp;
pub mod textures;
