//! Cutting the ground into square cells: each cell becomes one section file of the game, which streams in and out by the
//! camera's distance to its anchor (`docs/specs/second-world.md`). A triangle belongs to the cell its centroid is in.

use std::collections::HashMap;

use crate::pack::Pack;

/// Most vertices of one part: a solid's vertex buffer holds 65,536 and its indices are 16-bit.
pub const MAX_PART_VERTICES: usize = 60_000;

/// The triangles of one cell and one material, as a mesh of its own: 16-bit indices into its own vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub material: usize,
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub normals: Vec<[i8; 3]>,
    pub indices: Vec<u16>,
}

/// A triangle of ground a wheel can touch, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundTriangle {
    pub points: [[f32; 3]; 3],
    pub material: usize,
}

/// One cell: its meshes and its collision triangles.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The cell's column and row (`floor(x / size)`, `floor(y / size)`).
    pub key: (i32, i32),
    pub parts: Vec<Part>,
    pub ground: Vec<GroundTriangle>,
}

impl Cell {
    /// The sphere that holds every vertex of the cell: centre x, y, z and radius, render space.
    pub fn anchor(&self) -> [f32; 4] {
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for position in self.parts.iter().flat_map(|part| &part.positions) {
            for axis in 0..3 {
                low[axis] = low[axis].min(position[axis]);
                high[axis] = high[axis].max(position[axis]);
            }
        }
        if low[0] > high[0] {
            return [0.0, 0.0, 0.0, 1.0];
        }
        let centre = [0, 1, 2].map(|axis| (low[axis] + high[axis]) / 2.0);
        let radius = (0..3).map(|axis| ((high[axis] - low[axis]) / 2.0).powi(2)).sum::<f32>().sqrt();
        [centre[0], centre[1], centre[2], radius.max(1.0)]
    }
}

struct PartBuilder {
    part: Part,
    remap: HashMap<u32, u16>,
}

#[derive(Default)]
struct CellBuilder {
    /// The part each group is filling now (an index into `parts`).
    open: HashMap<usize, usize>,
    parts: Vec<PartBuilder>,
    ground: Vec<GroundTriangle>,
}

/// The cells of `pack`, `size` metres wide, in column then row order.
pub fn bin(pack: &Pack, size: f32) -> Vec<Cell> {
    let mut cells: HashMap<(i32, i32), CellBuilder> = HashMap::new();
    for (group_index, group) in pack.groups.iter().enumerate() {
        for triangle in group.indices.chunks_exact(3) {
            let corners = [triangle[0] as usize, triangle[1] as usize, triangle[2] as usize];
            let points = corners.map(|c| group.positions[c]);
            let x = (points[0][0] + points[1][0] + points[2][0]) / 3.0;
            let y = (points[0][1] + points[1][1] + points[2][1]) / 3.0;
            let key = ((x / size).floor() as i32, (y / size).floor() as i32);
            let cell = cells.entry(key).or_default();
            if group.collide {
                cell.ground.push(GroundTriangle { points, material: group.material });
            }
            let at = open_part(cell, group_index, group.material);
            let builder = &mut cell.parts[at];
            for corner in corners {
                let index = builder.vertex(group, corner);
                builder.part.indices.push(index);
            }
        }
    }
    let mut cells: Vec<Cell> = cells
        .into_iter()
        .map(|(key, builder)| Cell {
            key,
            parts: builder.parts.into_iter().map(|b| b.part).collect(),
            ground: builder.ground,
        })
        .collect();
    cells.sort_by_key(|cell| cell.key);
    cells
}

/// The part of `cell` that group `group` is filling, starting a new one when there is none or the open one is full.
fn open_part(cell: &mut CellBuilder, group: usize, material: usize) -> usize {
    if let Some(&at) = cell.open.get(&group)
        && cell.parts[at].part.positions.len() + 3 <= MAX_PART_VERTICES
    {
        return at;
    }
    let part = Part { material, positions: Vec::new(), uvs: Vec::new(), normals: Vec::new(), indices: Vec::new() };
    cell.parts.push(PartBuilder { part, remap: HashMap::new() });
    let at = cell.parts.len() - 1;
    cell.open.insert(group, at);
    at
}

impl PartBuilder {
    /// The index of the source vertex `source` in this part, copying it in the first time.
    fn vertex(&mut self, group: &crate::pack::Group, corner: usize) -> u16 {
        let source = corner as u32;
        if let Some(&index) = self.remap.get(&source) {
            return index;
        }
        let index = self.part.positions.len() as u16;
        self.part.positions.push(group.positions[corner]);
        self.part.uvs.push(group.uvs[corner]);
        self.part.normals.push(group.normals[corner]);
        self.remap.insert(source, index);
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{Group, Lap, Material};

    fn material() -> Material {
        Material { name: "m".into(), texture: None, color: [1, 2, 3], surface: "asphalt".into() }
    }

    /// A grid of `n` by `n` unit squares (two triangles each) from the origin, at height 1.
    fn grid(n: usize, collide: bool) -> Group {
        let mut positions = Vec::new();
        for y in 0..=n {
            for x in 0..=n {
                positions.push([x as f32, y as f32, 1.0]);
            }
        }
        let mut indices = Vec::new();
        for y in 0..n {
            for x in 0..n {
                let a = (y * (n + 1) + x) as u32;
                let b = a + 1;
                let c = a + (n + 1) as u32;
                let d = c + 1;
                indices.extend([a, b, d, a, d, c]);
            }
        }
        let count = positions.len();
        Group {
            material: 0,
            collide,
            positions,
            uvs: vec![[0.0, 0.0]; count],
            normals: vec![[0, 0, 127]; count],
            indices,
        }
    }

    fn pack(groups: Vec<Group>) -> Pack {
        Pack { track: "T".into(), materials: vec![material()], groups, lap: Lap::default() }
    }

    #[test]
    fn triangles_go_to_the_cell_of_their_centroid_and_every_triangle_is_kept() {
        let cells = bin(&pack(vec![grid(8, true)]), 4.0);
        assert_eq!(cells.len(), 4);
        let triangles: usize = cells.iter().map(|c| c.parts.iter().map(|p| p.indices.len() / 3).sum::<usize>()).sum();
        assert_eq!(triangles, 8 * 8 * 2);
        let ground: usize = cells.iter().map(|c| c.ground.len()).sum();
        assert_eq!(ground, 128);
        assert_eq!(cells[0].key, (0, 0));
        for cell in &cells {
            for part in &cell.parts {
                assert!(part.indices.iter().all(|&i| usize::from(i) < part.positions.len()));
            }
        }
    }

    #[test]
    fn a_group_that_is_not_collision_adds_no_ground_and_an_anchor_holds_the_cell() {
        let cells = bin(&pack(vec![grid(4, false)]), 100.0);
        assert_eq!(cells.len(), 1);
        assert!(cells[0].ground.is_empty());
        let [x, y, z, r] = cells[0].anchor();
        assert_eq!((x, y, z), (2.0, 2.0, 1.0));
        assert!((r - 8.0_f32.sqrt()).abs() < 1e-5, "radius {r}");
    }

    #[test]
    fn a_part_that_would_overflow_its_vertex_buffer_starts_a_new_one() {
        // 250 by 250 squares is 63,001 vertices in one cell: two parts.
        let cells = bin(&pack(vec![grid(250, false)]), 1000.0);
        assert_eq!(cells.len(), 1);
        assert!(cells[0].parts.len() >= 2, "{} parts", cells[0].parts.len());
        assert!(cells[0].parts.iter().all(|p| p.positions.len() <= MAX_PART_VERTICES));
        let triangles: usize = cells[0].parts.iter().map(|p| p.indices.len() / 3).sum();
        assert_eq!(triangles, 250 * 250 * 2);
    }
}
