//! Glue between the EA Black Box format readers and `blackbox-render`:
//! uploading [`blackbox_tpk::Texture`]s and [`blackbox_solid::Solid`]s, and the
//! small amount of geometry math scenes need (bounding boxes, frustum culling).
//! Game-agnostic.

mod aabb;
mod frustum;
mod meshes;
mod textures;

pub use aabb::Aabb;
pub use frustum::Frustum;
pub use meshes::{MaterialLookup, solid_mesh, upload_solid};
pub use textures::{blend_mode, upload_texture};
