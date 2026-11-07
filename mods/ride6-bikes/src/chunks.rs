//! Shading groups packed into as many solids as a body needs.
//!
//! A solid holds at most [`MAX_SOLID_VERTICES`] vertices and [`MAX_SOLID_TRIANGLES`] triangles (one 16-bit index buffer per effect).
//! The frame of a bike at the finest level of detail has 100,000 to 150,000 vertices, so it is several solids: the body solid and
//! `<ID>_KIT00_BODY<n>_<L>` for the rest, which the car's manifest binds as base parts (`docs/modding/api-content.md#components`).
//! Before this the frame had to fit one solid and the importer fell back to a coarser level of detail (a quarter of the triangles of
//! the engine for a 690).

use std::collections::HashMap;

use blackbox_solid::GroupSource;

use crate::build::{MAX_SOLID_TRIANGLES, MAX_SOLID_VERTICES};

/// The solids of one body at one level: more than this and the level is not used.
pub const MAX_BODY_SOLIDS: usize = 6;

/// `group` as pieces that each fit a solid. A group that fits is returned as it is; a larger one is cut along its triangle list, each
/// piece with the vertices its triangles use.
pub fn split_group(group: GroupSource) -> Vec<GroupSource> {
    let triangles = group.indices.len() / 3;
    if group.vertices.len() <= MAX_SOLID_VERTICES && triangles <= MAX_SOLID_TRIANGLES {
        return vec![group];
    }
    let mut pieces = Vec::new();
    let mut piece = Piece::default();
    for corners in group.indices.as_chunks::<3>().0 {
        let fresh = corners.iter().filter(|c| !piece.remap.contains_key(c)).count();
        if piece.vertices.len() + fresh > MAX_SOLID_VERTICES || piece.indices.len() / 3 + 1 > MAX_SOLID_TRIANGLES {
            pieces.push(piece.finish(&group));
            piece = Piece::default();
        }
        for &corner in corners {
            piece.add(corner);
        }
    }
    if !piece.indices.is_empty() {
        pieces.push(piece.finish(&group));
    }
    pieces
}

/// The vertices and indices of one piece of a split group.
#[derive(Default)]
struct Piece {
    remap: HashMap<u16, u16>,
    vertices: Vec<usize>,
    indices: Vec<u16>,
}

impl Piece {
    fn add(&mut self, corner: u16) {
        let next = self.vertices.len();
        let at = *self.remap.entry(corner).or_insert_with(|| {
            self.vertices.push(usize::from(corner));
            next as u16
        });
        self.indices.push(at);
    }

    /// A group with `group`'s shading and this piece's vertices and triangles.
    fn finish(self, group: &GroupSource) -> GroupSource {
        let tangents = match group.tangents.is_empty() {
            true => Vec::new(),
            false => self.vertices.iter().map(|&v| group.tangents[v]).collect(),
        };
        GroupSource {
            texture_slots: group.texture_slots,
            light_material: group.light_material,
            effect_id: group.effect_id,
            flags: group.flags,
            material_name: group.material_name.clone(),
            bounds: None,
            vertices: self.vertices.iter().map(|&v| group.vertices[v]).collect(),
            tangents,
            indices: self.indices,
        }
    }
}

/// `groups` packed in order into solids that each fit: the groups of a solid are consecutive, and a group too large for one solid
/// is cut ([`split_group`]). Never empty for a non-empty input.
pub fn pack(groups: Vec<GroupSource>) -> Vec<Vec<GroupSource>> {
    let mut solids: Vec<Vec<GroupSource>> = Vec::new();
    let (mut vertices, mut triangles) = (0usize, 0usize);
    for group in groups.into_iter().flat_map(split_group) {
        let (v, t) = (group.vertices.len(), group.indices.len() / 3);
        let fits = vertices + v <= MAX_SOLID_VERTICES && triangles + t <= MAX_SOLID_TRIANGLES;
        match solids.last_mut() {
            Some(solid) if fits => {
                solid.push(group);
                vertices += v;
                triangles += t;
            }
            _ => {
                solids.push(vec![group]);
                (vertices, triangles) = (v, t);
            }
        }
    }
    solids
}

#[cfg(test)]
mod tests {
    use blackbox_solid::Vertex;

    use super::*;

    fn group(vertices: usize, triangles: usize) -> GroupSource {
        let vertex = |i: usize| Vertex {
            position: [i as f32, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            color_bgra: [255; 4],
            uv: [0.0; 2],
        };
        // Triangle t uses vertices t, t + 1 and t + 2 (wrapping), so neighbours share vertices.
        let indices: Vec<u16> = (0..triangles)
            .flat_map(|t| [t % vertices, (t + 1) % vertices, (t + 2) % vertices])
            .map(|i| i as u16)
            .collect();
        let mut g = GroupSource::new((0..vertices).map(vertex).collect(), indices);
        g.light_material = 3;
        g.texture_slots = [1, 2, 3, 4, 5];
        g
    }

    #[test]
    fn a_group_that_fits_is_not_cut() {
        let pieces = split_group(group(100, 90));
        assert_eq!(pieces.len(), 1);
        assert_eq!((pieces[0].vertices.len(), pieces[0].indices.len()), (100, 270));
    }

    #[test]
    fn a_large_group_is_cut_into_pieces_that_keep_every_triangle_and_its_shading() {
        let big = group(65_000, 130_000);
        let positions: Vec<[f32; 3]> = big
            .indices
            .chunks_exact(3)
            .flat_map(|c| c.iter().map(|&i| big.vertices[usize::from(i)].position).collect::<Vec<_>>())
            .collect();
        let pieces = split_group(big);
        assert!(pieces.len() >= 3, "{}", pieces.len());
        let mut seen = Vec::new();
        for piece in &pieces {
            assert!(piece.vertices.len() <= MAX_SOLID_VERTICES && piece.indices.len() / 3 <= MAX_SOLID_TRIANGLES);
            assert_eq!((piece.light_material, piece.texture_slots), (3, [1, 2, 3, 4, 5]));
            seen.extend(piece.indices.iter().map(|&i| piece.vertices[usize::from(i)].position));
        }
        assert_eq!(seen, positions, "the triangles come back in order with their vertices");
    }

    #[test]
    fn groups_are_packed_in_order_into_solids_that_fit() {
        let solids = pack(vec![group(40_000, 20_000), group(30_000, 20_000), group(50_000, 30_000), group(10, 5)]);
        assert_eq!(solids.len(), 3);
        assert_eq!(solids[0].len(), 1);
        assert_eq!(solids[2].len(), 2, "a small group joins the last solid");
        assert!(solids.iter().all(|s| s.iter().map(|g| g.vertices.len()).sum::<usize>() <= MAX_SOLID_VERTICES));
        assert!(pack(Vec::new()).is_empty());
    }

    #[test]
    fn a_group_over_the_limits_is_cut_while_packing() {
        let solids = pack(vec![group(65_000, 140_000)]);
        assert!(solids.len() >= 3);
        assert!(solids.iter().all(|s| s.iter().map(|g| g.indices.len() / 3).sum::<usize>() <= MAX_SOLID_TRIANGLES));
    }
}
