//! Walking `GeometryPack`s and reading each `SolidPack`.

mod mesh;
mod solid;

use blackbox_chunk::{Chunk, ids};

use crate::{Result, Solid};

pub use solid::read_solid;

/// Every solid in every `GeometryPack` of `data`.
pub fn read_solids(data: &[u8]) -> Result<Vec<Solid>> {
    let mut solids = Vec::new();
    for pack in blackbox_chunk::find_all(data, ids::GEOMETRY_PACK) {
        read_pack_children(pack, &mut solids)?;
    }
    Ok(solids)
}

fn read_pack_children(pack: Chunk<'_>, out: &mut Vec<Solid>) -> Result<()> {
    for child in pack.children() {
        let child = child?;
        if child.id == ids::SOLID_PACK {
            out.push(read_solid(child)?);
        } else if child.is_bare_jdlz() {
            // A compressed SolidPack (community-built add-on cars). Offsets inside are blob-relative.
            let inflated = ea_compress::jdlz_decompress(child.payload)?;
            for inner in blackbox_chunk::chunks(&inflated) {
                let inner = inner?;
                if inner.id == ids::SOLID_PACK {
                    out.push(read_solid(inner)?);
                }
            }
        }
    }
    Ok(())
}
