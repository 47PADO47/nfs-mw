//! Writing solids back: a `GeometryPack` (`CARS/<CAR>/GEOMETRY.BIN`) built from plain vertex and index lists.
//! Spec: `docs/formats/models.md`; the reader in [`crate::read_solids`] is the oracle (what is written reads back as
//! the same data).
//!
//! What comes out follows the stock car files: a `MeshContainerInfo` with the pack header, the keys and the offsets
//! (both sorted by name hash), then one `SolidPack` per solid on a 0x80 boundary, each with `SolidInfo` (version
//! 0x16), the texture and light-material hash lists, the markers, and a `MeshInfoContainer` of 104-byte shading
//! groups, `u16` indices and one 36-byte-per-vertex buffer per run of groups with the same effect id.
//!
//! Not written: skinned, tangent or second-UV vertices (see [`GroupSource`] for the effect ids that are safe), the
//! normal-smoothing chunks (`00134017` to `00134019`) and the `SolidInfo` bone count.

mod error;
mod file;
mod from_solid;
mod info;
mod mesh;
mod plan;
mod put;
mod solid_pack;
mod source;
mod validate;

use blackbox_hash::bstring_hash;

use self::file::{BuiltPack, assemble};
use self::solid_pack::build_solid_pack;
use self::validate::validate;
use crate::layout;

pub use error::{GroupProblem, WriteError};
pub use source::{
    BOUNDS_MARGIN, CAR_EFFECT_ID, COMMON_GROUP_FLAGS, DEFAULT_SOLID_FLAGS, GroupSource, IDENTITY, SolidSource,
    padded_bounds,
};

/// A `GEOMETRY.BIN` holding these solids, in this order. The mesh container's tables list them sorted by name hash.
///
/// Fails, before anything is built, on a solid the reader would not take back: no groups, a group without vertices
/// or triangles, an index past the group's vertices, a wide effect id, more than 65536 vertices in one vertex buffer,
/// or two solids with the same name hash.
pub fn write_geometry_pack(solids: &[SolidSource]) -> Result<Vec<u8>, WriteError> {
    // Version 0x16: NFS: Most Wanted (PC).
    let layout = &layout::V16;
    validate(solids, layout)?;
    let mut packs = Vec::with_capacity(solids.len());
    for solid in solids {
        packs.push(BuiltPack { hash: bstring_hash(&solid.name), bytes: build_solid_pack(solid, layout)? });
    }
    assemble(&packs)
}
