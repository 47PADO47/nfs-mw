//! Walking `GeometryPack`s and reading each `SolidPack`.

mod mesh;
mod skin;
mod solid;

use blackbox_chunk::{Chunk, ids};

use crate::{Error, Result, Solid};

pub use solid::read_solid;

/// Every solid in every `GeometryPack` of `data`, then every solid that stands on its own at the top level of the file.
///
/// Some community-built cars (the install's `CORVETTE`, `COBALTSS`, `TT`, `BMWM3`) keep only the mesh tables in their
/// `GeometryPack` and store each `SolidPack` after it, as a sibling. Fails on the first solid that does not parse; see
/// [`read_solids_lossy`] to keep the rest.
pub fn read_solids(data: &[u8]) -> Result<Vec<Solid>> {
    let (solids, mut failed) = read_solids_lossy(data)?;
    match failed.is_empty() {
        true => Ok(solids),
        false => Err(failed.swap_remove(0)),
    }
}

/// Like [`read_solids`], but a solid that does not parse is left out and its error returned beside the solids that did.
/// Add-on cars can carry a few broken solids (the install's `CORVETTE` has 8 aftermarket hoods with a vertex buffer that
/// fits no vertex count); the rest of the car is still good. A file whose chunks cannot be walked is still an error.
pub fn read_solids_lossy(data: &[u8]) -> Result<(Vec<Solid>, Vec<Error>)> {
    let mut found = Found::default();
    for pack in blackbox_chunk::find_all(data, ids::GEOMETRY_PACK) {
        for child in pack.children() {
            read_member(child?, &mut found)?;
        }
    }
    for top in blackbox_chunk::chunks(data) {
        read_member(top?, &mut found)?;
    }
    Ok((found.solids, found.failed))
}

#[derive(Default)]
struct Found {
    solids: Vec<Solid>,
    failed: Vec<Error>,
}

impl Found {
    fn push(&mut self, solid: Result<Solid>) {
        match solid {
            Ok(solid) => self.solids.push(solid),
            Err(e) => self.failed.push(e),
        }
    }
}

/// A `SolidPack`, or a bare JDLZ blob that holds some, when `member` is one of them.
fn read_member(member: Chunk<'_>, out: &mut Found) -> Result<()> {
    if member.id == ids::SOLID_PACK {
        out.push(read_solid(member));
    } else if member.is_bare_jdlz() {
        // A compressed SolidPack (community-built add-on cars). Offsets inside are blob-relative.
        let inflated = ea_compress::jdlz_decompress(member.payload)?;
        for inner in blackbox_chunk::chunks(&inflated) {
            let inner = inner?;
            if inner.id == ids::SOLID_PACK {
                out.push(read_solid(inner));
            }
        }
    }
    Ok(())
}
