//! A model's parts as the game's solids: positions in render space around the solid's own centre (so the game can cull it by
//! its radius), normals worked out from the triangles (the 2012 world's vertices carry none: the lighting is in its maps), a
//! mid-grey vertex colour (the game's world is pre-lit and the colour doubles, so a middle value shows the picture as it is), and
//! the picture of each group as a texture of the solid.

use blackbox_hash::bstring_hash;
use blackbox_solid::{GroupSource, SolidSource, Vertex};

use crate::mesh::{MAX_PART, Part};
use crate::space;

/// The world shader of the game (`WorldShader`, 36-byte vertices): effect 0.
const WORLD_EFFECT: u32 = 0;
/// The vertex colour of a surface, as stored (blue, green, red, alpha).
const COLOUR: [u8; 4] = [0x58, 0x58, 0x58, 0xFF];
/// Whether the triangles are written the other way round to the 2012 order.
pub const FLIP_WINDING: bool = false;

/// A part of a model with the texture it is drawn with, in engine space.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// The game's name of the picture.
    pub texture: String,
    pub part: Part,
}

/// A solid made of groups, and where its origin is.
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltSolid {
    pub source: SolidSource,
    /// The solid's origin in the render space of the model: the vertices were moved by minus this.
    pub centre: [f32; 3],
    /// The distance from the origin to the farthest corner of the solid's box, metres.
    pub radius: f32,
}

/// The unit normals of the vertices of a part, from its triangles (the sum of the face normals, so a bigger face counts more).
pub fn normals(positions: &[[f32; 3]], indices: &[u16]) -> Vec<[f32; 3]> {
    let mut sums = vec![[0.0f32; 3]; positions.len()];
    for triangle in indices.chunks_exact(3) {
        let [a, b, c] = [0, 1, 2].map(|k| positions[usize::from(triangle[k])]);
        let (u, v) = ([0, 1, 2].map(|k| b[k] - a[k]), [0, 1, 2].map(|k| c[k] - a[k]));
        let face = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        for &index in triangle {
            for k in 0..3 {
                sums[usize::from(index)][k] += face[k];
            }
        }
    }
    sums.into_iter()
        .map(|[x, y, z]| {
            let length = (x * x + y * y + z * z).sqrt();
            if length > 1e-12 { [x / length, y / length, z / length] } else { [0.0, 0.0, 1.0] }
        })
        .collect()
}

/// The solids of `groups`, named `<prefix>0`, `<prefix>1`...: groups go together until a solid would hold more triangles than
/// its header can count. A model that needs more than one solid is placed once for each.
pub fn build(prefix: &str, groups: Vec<Group>) -> Vec<BuiltSolid> {
    let mut solids: Vec<Vec<Group>> = Vec::new();
    let mut triangles = 0;
    for group in groups {
        let count = group.part.indices.len() / 3;
        if solids.is_empty() || triangles + count > MAX_PART {
            solids.push(Vec::new());
            triangles = 0;
        }
        triangles += count;
        solids.last_mut().expect("a solid was just pushed").push(group);
    }
    solids.into_iter().enumerate().map(|(n, groups)| one(&format!("{prefix}{n}"), groups)).collect()
}

fn one(name: &str, groups: Vec<Group>) -> BuiltSolid {
    let render: Vec<Vec<[f32; 3]>> =
        groups.iter().map(|g| g.part.positions.iter().map(|&p| space::render(p)).collect()).collect();
    let mut low = [f32::INFINITY; 3];
    let mut high = [f32::NEG_INFINITY; 3];
    for position in render.iter().flatten() {
        for axis in 0..3 {
            low[axis] = low[axis].min(position[axis]);
            high[axis] = high[axis].max(position[axis]);
        }
    }
    let centre = [0, 1, 2].map(|axis| ((low[axis] + high[axis]) / 2.0).round());
    let radius = (0..3)
        .map(|axis| (low[axis] - centre[axis]).abs().max((high[axis] - centre[axis]).abs()).powi(2))
        .sum::<f32>()
        .sqrt();
    let mut hashes: Vec<u32> = Vec::new();
    let mut sources = Vec::with_capacity(groups.len());
    for (group, positions) in groups.iter().zip(&render) {
        let mut indices = group.part.indices.clone();
        if FLIP_WINDING {
            indices.as_chunks_mut::<3>().0.iter_mut().for_each(|triangle| triangle.swap(1, 2));
        }
        let moved: Vec<[f32; 3]> =
            positions.iter().map(|p| [p[0] - centre[0], p[1] - centre[1], p[2] - centre[2]]).collect();
        let normals = normals(&moved, &indices);
        let vertices: Vec<Vertex> = moved
            .iter()
            .zip(&normals)
            .zip(&group.part.uvs)
            .map(|((&position, &normal), &uv)| Vertex { position, normal, color_bgra: COLOUR, uv })
            .collect();
        let hash = bstring_hash(&group.texture);
        let slot = match hashes.iter().position(|&known| known == hash) {
            Some(slot) => slot,
            None => {
                hashes.push(hash);
                hashes.len() - 1
            }
        };
        let mut source = GroupSource::new(vertices, indices);
        source.effect_id = WORLD_EFFECT;
        source.flags = 0;
        source.texture_slots = [slot as u8; 5];
        sources.push(source);
    }
    let mut source = SolidSource::new(name, sources);
    source.texture_hashes = hashes;
    BuiltSolid { source, centre, radius }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle(texture: &str, offset: f32) -> Group {
        Group {
            texture: texture.to_owned(),
            part: Part {
                // Engine space: x left, y up, z forward; a triangle in the horizontal plane at height 2.
                positions: vec![[offset, 2.0, 0.0], [offset + 10.0, 2.0, 0.0], [offset, 2.0, 10.0]],
                uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                indices: vec![0, 1, 2],
            },
        }
    }

    #[test]
    fn the_normal_faces_the_side_the_corners_turn_counter_clockwise_to() {
        // Render space has z up. The corners (0,0,2), (0,10,2), (10,0,2) turn clockwise seen from above.
        let positions = [[0.0, 0.0, 2.0], [0.0, 10.0, 2.0], [10.0, 0.0, 2.0]];
        assert_eq!(normals(&positions, &[0, 1, 2])[0], [0.0, 0.0, -1.0]);
        assert_eq!(normals(&positions, &[0, 2, 1])[0], [0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_degenerate_triangle_gets_an_up_normal_and_normals_are_unit_length() {
        let n = normals(&[[0.0; 3], [0.0; 3], [0.0; 3]], &[0, 1, 2]);
        assert_eq!(n[1], [0.0, 0.0, 1.0]);
        let n = normals(&[[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [0.0, 4.0, 0.0]], &[0, 1, 2]);
        for normal in n {
            let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            assert!((length - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_solid_is_centred_on_its_box_with_a_radius_that_holds_it() {
        let solids = build("M7A", vec![triangle("F1", 100.0)]);
        assert_eq!(solids.len(), 1);
        let solid = &solids[0];
        assert_eq!(solid.source.name, "M7A0");
        // Engine box x 100..110, y 2, z 0..10 is render x 0..10, y 100..110, z 2: the centre is rounded to whole metres.
        assert_eq!(solid.centre, [5.0, 105.0, 2.0]);
        let half = ((5.0f32).powi(2) * 2.0).sqrt();
        assert!((solid.radius - half).abs() < 1e-4, "radius {}", solid.radius);
        let vertices = &solid.source.groups[0].vertices;
        assert_eq!(vertices[0].position, [-5.0, -5.0, 0.0]);
        assert_eq!(vertices[0].color_bgra, COLOUR);
    }

    #[test]
    fn groups_with_the_same_picture_share_a_texture_slot() {
        let solids = build("M1A", vec![triangle("F1", 0.0), triangle("F2", 0.0), triangle("F1", 5.0)]);
        let solid = &solids[0];
        assert_eq!(solid.source.texture_hashes, vec![bstring_hash("F1"), bstring_hash("F2")]);
        let slots: Vec<u8> = solid.source.groups.iter().map(|g| g.texture_slots[0]).collect();
        assert_eq!(slots, [0, 1, 0]);
    }

    #[test]
    fn too_many_triangles_make_a_second_solid_and_the_solids_read_back() {
        let big = Group {
            texture: "F9".to_owned(),
            part: Part {
                positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
                uvs: vec![[0.0, 0.0]; 3],
                indices: [0u16, 1, 2].repeat(40_000),
            },
        };
        let solids = build("M2A", vec![big.clone(), big]);
        assert_eq!(solids.len(), 2);
        let bytes = blackbox_solid::write_geometry_pack(&[solids[0].source.clone(), solids[1].source.clone()]).unwrap();
        let read = blackbox_solid::read_solids(&bytes).unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].name, "M2A0");
    }
}
