//! Solids ("models") from `GeometryPack` chunks in EA Black Box games.
//! Spec: `docs/formats/models.md`.
//!
//! Works on a byte slice that has already had any whole-file wrapper removed.
//! Bare JDLZ blobs standing in for `SolidPack` chunks (add-on cars built with
//! community tools) are inflated here.
//!
//! Layouts are selected by the `SolidInfo` version byte (see [`layout`]).
//! Supported today: version 0x16 (NFS: Most Wanted, PC).
//!
//! [`write_geometry_pack`] goes the other way: it builds a `GEOMETRY.BIN` from vertex and index lists, so a converter
//! can make car models.

mod bytes;
mod error;
pub mod layout;
mod model;
mod reader;
mod rigid;
mod writer;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use model::{BASE_VERTEX_STRIDE, PositionMarker, ShadingGroup, SkinVertex, Solid, Vertex, VertexBuffer};
pub use reader::{read_solid, read_solids, read_solids_filtered, read_solids_lossy};
pub use rigid::{PartRun, RigidPart, rigid_split};
pub use writer::{
    BOUNDS_MARGIN, CAR_EFFECT_ID, COMMON_GROUP_FLAGS, DEFAULT_SOLID_FLAGS, GroupProblem, GroupSource, IDENTITY,
    SolidSource, WriteError, padded_bounds, write_geometry_pack,
};
