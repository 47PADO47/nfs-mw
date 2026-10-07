//! Solids ("models") from `GeometryPack` chunks in EA Black Box games.
//! Spec: `docs/formats/models.md`.
//!
//! Works on a byte slice that has already had any whole-file wrapper removed.
//! Bare JDLZ blobs standing in for `SolidPack` chunks (add-on cars built with
//! community tools) are inflated here.
//!
//! Layouts are selected by the `SolidInfo` version byte (see [`layout`]).
//! Supported today: version 0x16 (NFS: Most Wanted, PC).

mod bytes;
mod error;
pub mod layout;
mod model;
mod reader;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use model::{BASE_VERTEX_STRIDE, ShadingGroup, Solid, Vertex, VertexBuffer};
pub use reader::{read_solid, read_solids};
