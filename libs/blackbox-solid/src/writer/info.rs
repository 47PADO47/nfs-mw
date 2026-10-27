//! The `SolidInfo` payload.

use blackbox_hash::bstring_hash;

use super::put::{put_f32, put_u16, put_u32, put_vec3};
use super::source::SolidSource;
use crate::layout::SolidLayout;

/// Offsets inside `SolidInfo` that the reader does not use. Checked on all 14,843 car solids that have a mesh.
/// NumBones, NumTextureTableEntries, NumLightMaterials and NumPositionMarkers, one byte each.
const COUNTS_AT: usize = 0x18;
const POINTER_PAIR_AT: usize = 0x88;
const VOLUME_AT: usize = 0x98;
/// The two u32s at 0x88 and 0x8C. Every car solid has 0x000EA550 in both. What they mean is not known; they are
/// written as the stock files have them.
const POINTER_PAIR: u32 = 0x000E_A550;

/// The payload (without the alignment fill): the fixed header, then the NUL-terminated name padded to 4 bytes, which
/// is how long every stock record is.
///
/// `triangles` is the polygon count at 0x14. NumVerts (0x16) is 0 like in every car file, and so are the bone count
/// and the marker count (the markers are counted from their chunk).
pub(super) fn info_payload(solid: &SolidSource, triangles: u16, layout: &SolidLayout) -> Vec<u8> {
    let l = &layout.info;
    let name = solid.name.as_bytes();
    let mut b = vec![0u8; l.name + (name.len() + 1).next_multiple_of(4)];
    b[l.version] = layout.version;
    put_u16(&mut b, l.flags, solid.flags);
    put_u32(&mut b, l.name_hash, bstring_hash(&solid.name));
    put_u16(&mut b, l.num_polys, triangles);
    // Both lists are at most 255 long (checked before).
    b[COUNTS_AT + 1] = solid.texture_hashes.len() as u8;
    b[COUNTS_AT + 2] = solid.light_material_hashes.len() as u8;
    put_vec3(&mut b, l.bounds_min, solid.bounds_min);
    put_vec3(&mut b, l.bounds_max, solid.bounds_max);
    for (i, f) in solid.transform.into_iter().enumerate() {
        put_f32(&mut b, l.transform + i * 4, f);
    }
    put_u32(&mut b, POINTER_PAIR_AT, POINTER_PAIR);
    put_u32(&mut b, POINTER_PAIR_AT + 4, POINTER_PAIR);
    // Stock files: the volume of the bounds at 0x98, then polygons per unit of volume. The product is formed in double
    // precision and rounded once (the stock values differ from an f32 product in the last digit).
    let extent = |a: usize| f64::from(solid.bounds_max[a]) - f64::from(solid.bounds_min[a]);
    let volume = (extent(0) * extent(1) * extent(2)) as f32;
    put_f32(&mut b, VOLUME_AT, volume);
    let density = solid.density.unwrap_or(match volume > 0.0 {
        true => f32::from(triangles) / volume,
        false => 0.0,
    });
    put_f32(&mut b, l.density, density);
    b[l.name..l.name + name.len()].copy_from_slice(name);
    b
}
