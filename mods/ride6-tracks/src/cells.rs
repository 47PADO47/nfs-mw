//! Cutting the ground into square cells: each cell becomes one section file of the game, which streams in and out by the
//! camera's distance to its anchor (`docs/specs/second-world.md`). A triangle belongs to the cell its centroid is in.

use std::collections::HashMap;

use crate::pack::Pack;

/// Most vertices of one part: a solid's vertex buffer holds 65,536 and its indices are 16-bit.
pub const MAX_PART_VERTICES: usize = 60_000;
/// Most triangles of one part: a solid's triangle count is 16-bit.
pub const MAX_PART_TRIANGLES: usize = 60_000;

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

/// How the ground is cut into cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// A cell holding more triangles than this is cut in four, unless it is already as small as `min_size`.
    pub max_triangles: usize,
    /// The side of the smallest cell, metres.
    pub min_size: f32,
}

/// Which parts of a pack an import keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keep {
    /// The barriers, buildings and props, and the walls they make in the collision.
    pub objects: bool,
    /// The racing line as a ribbon on the ground.
    pub line: bool,
}

impl Default for Keep {
    fn default() -> Self {
        Self { objects: true, line: true }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_triangles: 40_000, min_size: 128.0 }
    }
}

/// One triangle of the pack: its group and its first index there.
#[derive(Clone, Copy)]
struct TriangleRef {
    group: u32,
    first: u32,
}

/// The cells of `pack` in a quadtree over the ground: a square of ground with more than `limits.max_triangles` triangles is cut in
/// four, so dense ground (the circuit) gets small cells and the far hills a few large ones. A triangle belongs to the cell its
/// centroid is in. The cells come in the order of their corners (column, then row).
pub fn bin(pack: &Pack, limits: &Limits) -> Vec<Cell> {
    let mut refs = Vec::new();
    for (group, g) in pack.groups.iter().enumerate() {
        refs.extend((0..g.indices.len() / 3).map(|t| TriangleRef { group: group as u32, first: (t * 3) as u32 }));
    }
    if refs.is_empty() {
        return Vec::new();
    }
    let (mut low, mut high) = ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]);
    for &triangle in &refs {
        let [x, y] = centroid(pack, triangle);
        low = [low[0].min(x), low[1].min(y)];
        high = [high[0].max(x), high[1].max(y)];
    }
    let mut size = limits.min_size;
    while size < (high[0] - low[0]).max(high[1] - low[1]) + 1.0 {
        size *= 2.0;
    }
    let mut leaves = Vec::new();
    split(pack, limits, ([low[0], low[1]], size), refs, &mut leaves);
    let mut cells: Vec<Cell> =
        leaves.into_iter().map(|(corner, triangles)| build(pack, corner, &triangles, limits)).collect();
    cells.sort_by_key(|cell| cell.key);
    cells
}

fn centroid(pack: &Pack, triangle: TriangleRef) -> [f32; 2] {
    let group = &pack.groups[triangle.group as usize];
    let at = triangle.first as usize;
    let p = |k: usize| group.positions[group.indices[at + k] as usize];
    [(p(0)[0] + p(1)[0] + p(2)[0]) / 3.0, (p(0)[1] + p(1)[1] + p(2)[1]) / 3.0]
}

/// Cuts the square `(corner, size)` holding `triangles` until each piece is small enough; the pieces go to `leaves` with their corners.
fn split(
    pack: &Pack,
    limits: &Limits,
    (corner, size): ([f32; 2], f32),
    triangles: Vec<TriangleRef>,
    leaves: &mut Vec<([f32; 2], Vec<TriangleRef>)>,
) {
    if triangles.is_empty() {
        return;
    }
    if triangles.len() <= limits.max_triangles || size <= limits.min_size {
        leaves.push((corner, triangles));
        return;
    }
    let half = size / 2.0;
    let mut quarters: [Vec<TriangleRef>; 4] = Default::default();
    for triangle in triangles {
        let [x, y] = centroid(pack, triangle);
        let (east, north) = (x >= corner[0] + half, y >= corner[1] + half);
        quarters[usize::from(east) + 2 * usize::from(north)].push(triangle);
    }
    for (k, quarter) in quarters.into_iter().enumerate() {
        let origin = [corner[0] + half * (k % 2) as f32, corner[1] + half * (k / 2) as f32];
        split(pack, limits, (origin, half), quarter, leaves);
    }
}

/// The cell of one leaf: its parts, by group, and its ground.
fn build(pack: &Pack, corner: [f32; 2], triangles: &[TriangleRef], limits: &Limits) -> Cell {
    let mut cell = CellBuilder::default();
    for &triangle in triangles {
        let group = &pack.groups[triangle.group as usize];
        let at = triangle.first as usize;
        let corners = [0, 1, 2].map(|k| group.indices[at + k] as usize);
        if group.collide {
            cell.ground.push(GroundTriangle { points: corners.map(|c| group.positions[c]), material: group.material });
        }
        let part = open_part(&mut cell, triangle.group as usize, group.material);
        let builder = &mut cell.parts[part];
        for corner in corners {
            let index = builder.vertex(group, corner);
            builder.part.indices.push(index);
        }
    }
    let key = ((corner[0] / limits.min_size).floor() as i32, (corner[1] / limits.min_size).floor() as i32);
    Cell { key, parts: cell.parts.into_iter().map(|b| b.part).collect(), ground: cell.ground }
}

/// The parts of a model's group, split where its vertices or triangles would not fit a solid.
pub fn parts_of(group: &crate::pack::Group) -> Vec<Part> {
    let mut cell = CellBuilder::default();
    for triangle in group.indices.as_chunks::<3>().0 {
        let at = open_part(&mut cell, 0, group.material);
        let builder = &mut cell.parts[at];
        for &corner in triangle {
            let index = builder.vertex(group, corner as usize);
            builder.part.indices.push(index);
        }
    }
    cell.parts.into_iter().map(|b| b.part).collect()
}

/// The part of `cell` that group `group` is filling, starting a new one when there is none or the open one is full.
fn open_part(cell: &mut CellBuilder, group: usize, material: usize) -> usize {
    if let Some(&at) = cell.open.get(&group)
        && cell.parts[at].part.positions.len() + 3 <= MAX_PART_VERTICES
        && cell.parts[at].part.indices.len() / 3 < MAX_PART_TRIANGLES
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
            model: 0,
            positions,
            uvs: vec![[0.0, 0.0]; count],
            normals: vec![[0, 0, 127]; count],
            indices,
        }
    }

    fn pack(groups: Vec<Group>) -> Pack {
        Pack {
            track: "T".into(),
            materials: vec![material()],
            groups,
            lap: Lap::default(),
            sea: None,
            models: vec![],
            object_groups: vec![],
            instances: vec![],
        }
    }

    #[test]
    fn triangles_go_to_the_cell_of_their_centroid_and_every_triangle_is_kept() {
        let cells = bin(&pack(vec![grid(8, true)]), &Limits { max_triangles: 40, min_size: 4.0 });
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
        let cells = bin(&pack(vec![grid(4, false)]), &Limits::default());
        assert_eq!(cells.len(), 1);
        assert!(cells[0].ground.is_empty());
        let [x, y, z, r] = cells[0].anchor();
        assert_eq!((x, y, z), (2.0, 2.0, 1.0));
        assert!((r - 8.0_f32.sqrt()).abs() < 1e-5, "radius {r}");
    }

    #[test]
    fn a_part_that_would_overflow_its_vertex_buffer_starts_a_new_one() {
        // 250 by 250 squares is 63,001 vertices in one cell: two parts.
        let cells = bin(&pack(vec![grid(250, false)]), &Limits { max_triangles: 1_000_000, min_size: 1000.0 });
        assert_eq!(cells.len(), 1);
        assert!(cells[0].parts.len() >= 2, "{} parts", cells[0].parts.len());
        assert!(cells[0].parts.iter().all(|p| p.positions.len() <= MAX_PART_VERTICES));
        let triangles: usize = cells[0].parts.iter().map(|p| p.indices.len() / 3).sum();
        assert_eq!(triangles, 250 * 250 * 2);
    }
}
