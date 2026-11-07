//! A cell's parts as the game's solids: positions around the solid's own centre, a mid-grey vertex colour (the world is
//! pre-lit: the colour doubles, so 0x80 is "as the picture is"), the picture as texture 0 of a world shader group.

use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, SolidSource, Vertex};

use crate::cells::Part;

/// The world shader of the game (`WorldShader`, 36-byte vertices): effect 0.
const WORLD_EFFECT: u32 = 0;
/// The vertex colour of the ground, as stored (blue, green, red, alpha).
const GROUND_COLOUR: [u8; 4] = [0x80, 0x80, 0x80, 0xFF];
/// Whether the triangles are written in the opposite order to the pack's (counter-clockwise from outside): the game draws
/// the other way round.
pub const FLIP_WINDING: bool = false;

/// A solid made of one part, and where to place it.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedSolid {
    pub source: SolidSource,
    /// The solid's centre in render space: the placement position.
    pub centre: [f32; 3],
}

/// The solid of `part`, named `name`, textured with the texture called `texture`.
pub fn solid_of(name: &str, part: &Part, texture: &str) -> PlacedSolid {
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for position in &part.positions {
        for axis in 0..3 {
            low[axis] = low[axis].min(position[axis]);
            high[axis] = high[axis].max(position[axis]);
        }
    }
    let centre = [0, 1, 2].map(|axis| ((low[axis] + high[axis]) / 2.0).round());
    let vertices: Vec<Vertex> = part
        .positions
        .iter()
        .zip(&part.uvs)
        .zip(&part.normals)
        .map(|((position, uv), normal)| Vertex {
            position: [position[0] - centre[0], position[1] - centre[1], position[2] - centre[2]],
            normal: normal.map(|n| f32::from(n) / 127.0),
            color_bgra: GROUND_COLOUR,
            uv: *uv,
        })
        .collect();
    let mut indices = part.indices.clone();
    if FLIP_WINDING {
        indices.chunks_exact_mut(3).for_each(|triangle| triangle.swap(1, 2));
    }
    let mut group = GroupSource::new(vertices, indices);
    group.effect_id = WORLD_EFFECT;
    group.flags = 0;
    let mut source = SolidSource::new(name, vec![group]);
    source.texture_hashes = vec![bstring_hash(texture)];
    PlacedSolid { source, centre }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part() -> Part {
        Part {
            material: 0,
            positions: vec![[100.0, 200.0, 5.0], [110.0, 200.0, 5.0], [110.0, 210.0, 9.0]],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            normals: vec![[0, 0, 127]; 3],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn a_part_is_centred_on_its_box_and_placed_there() {
        let placed = solid_of("G000_0", &part(), "T_GRASS");
        assert_eq!(placed.centre, [105.0, 205.0, 7.0]);
        let positions: Vec<_> = placed.source.groups[0].vertices.iter().map(|v| v.position).collect();
        assert_eq!(positions[0], [-5.0, -5.0, -2.0]);
        assert_eq!(positions[2], [5.0, 5.0, 2.0]);
        assert_eq!(placed.source.texture_hashes, vec![bstring_hash("T_GRASS")]);
        assert_eq!(placed.source.groups[0].effect_id, 0);
    }

    #[test]
    fn the_winding_is_turned_when_asked_and_the_solid_writes_and_reads_back() {
        let placed = solid_of("G000_0", &part(), "T_GRASS");
        let expected: Vec<u16> = if FLIP_WINDING { vec![0, 2, 1] } else { vec![0, 1, 2] };
        assert_eq!(placed.source.groups[0].indices, expected);
        let bytes = blackbox_solid::write_geometry_pack(&[placed.source]).unwrap();
        let solids = blackbox_solid::read_solids(&bytes).unwrap();
        assert_eq!(solids.len(), 1);
        assert_eq!(solids[0].name, "G000_0");
        assert_eq!(solids[0].groups.len(), 1);
    }
}
