//! The ground of a section: the triangles of its terrain in a plan grid, for "how high is the ground at this spot".
//!
//! The 2005 world's terrain (the hills, the verges, the lots and the road surface the player sees) is the solids named `TRN_*` of each tile,
//! in world coordinates like every tile solid (a tile solid is drawn without its pivot). The collision of a tile holds the roads and the
//! walls but not the hills, so it cannot say where a tree line stands (`docs/formats/chameleon-vegetation.md`, "Standing on the ground");
//! the visible terrain can. [`Terrain::surface`] asks for the surface at a spot **near a height**: a tile has bridges and walls over its
//! ground, so there is more than one surface at a spot, and the one nearest the height asked for (within a window) is the ground a tree
//! at that height stands on.

use blackbox_solid::Solid;

/// What the terrain solids of a tile begin with (upper case).
pub const TERRAIN_PREFIX: &str = "TRN_";
/// The side of a cell of the plan grid, metres.
const CELL: f32 = 16.0;
/// A point on the edge of a triangle counts as inside (metres of slack in the barycentric test).
const EDGE_SLACK: f32 = 1e-4;
/// Two surfaces this close to the height asked for are equally near, metres.
const TIE: f32 = 1e-4;

/// One triangle of the terrain, its corners x, y, z in world coordinates (z is up).
pub type Triangle = [[f32; 3]; 3];

/// The terrain triangles of a section.
#[derive(Debug, Default)]
pub struct Terrain {
    origin: [f32; 2],
    cols: usize,
    rows: usize,
    /// For each cell, the triangles whose box touches it.
    cells: Vec<Vec<u32>>,
    triangles: Vec<Triangle>,
}

/// Whether a solid of this name is terrain.
pub fn is_terrain(name: &str) -> bool {
    name.get(..TERRAIN_PREFIX.len()).is_some_and(|start| start.eq_ignore_ascii_case(TERRAIN_PREFIX))
}

/// Whether a solid is a world-space copy of a wall or a building (`XW_`, `XB_`): the top of one holds up a tree that stands on it (a tree
/// line whose crowns begin at a wall's top), so its triangles are ground too. The instanced ones are [`Neighbours::supports`](super::clear).
fn is_support(name: &str) -> bool {
    super::clear::is_obstacle(name) && super::active::world_copy_base(name).is_some()
}

/// The triangles of the groups of a solid, in the solid's own coordinates.
fn triangles_of(solid: &Solid, out: &mut Vec<Triangle>) {
    for group in &solid.groups {
        let first = group.first_index as usize;
        let Some(indices) = solid.indices.get(first..first + group.num_indices as usize) else { continue };
        for corners in indices.chunks_exact(3) {
            let at =
                |index: u16| solid.vertices.get(group.base_vertex as usize + usize::from(index)).map(|v| v.position);
            let (Some(a), Some(b), Some(c)) = (at(corners[0]), at(corners[1]), at(corners[2])) else { continue };
            out.push([a, b, c]);
        }
    }
}

impl Terrain {
    /// The terrain of the solids of a section; `None` when it has none.
    pub fn of_solids(solids: &[Solid]) -> Option<Self> {
        let mut triangles = Vec::new();
        for solid in solids.iter().filter(|solid| is_terrain(&solid.name) || is_support(&solid.name)) {
            triangles_of(solid, &mut triangles);
        }
        Self::from_triangles(triangles)
    }

    /// The terrain made of `triangles`; `None` when there are none or a corner is not a number.
    pub fn from_triangles(triangles: Vec<Triangle>) -> Option<Self> {
        let finite = |t: &Triangle| t.iter().all(|p| p.iter().all(|v| v.is_finite()));
        let triangles: Vec<Triangle> = triangles.into_iter().filter(finite).collect();
        if triangles.is_empty() {
            return None;
        }
        let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
        for corner in triangles.iter().flatten() {
            low = [low[0].min(corner[0]), low[1].min(corner[1])];
            high = [high[0].max(corner[0]), high[1].max(corner[1])];
        }
        let cols = (((high[0] - low[0]) / CELL) as usize + 1).max(1);
        let rows = (((high[1] - low[1]) / CELL) as usize + 1).max(1);
        let mut out = Self { origin: low, cols, rows, cells: vec![Vec::new(); cols * rows], triangles };
        for (number, triangle) in out.triangles.iter().enumerate() {
            let (mut tl, mut th) = ([f32::MAX; 2], [f32::MIN; 2]);
            for corner in triangle {
                tl = [tl[0].min(corner[0]), tl[1].min(corner[1])];
                th = [th[0].max(corner[0]), th[1].max(corner[1])];
            }
            let ((c0, r0), (c1, r1)) = (out.cell_of(tl), out.cell_of(th));
            for row in r0..=r1 {
                for col in c0..=c1 {
                    out.cells[row * cols + col].push(number as u32);
                }
            }
        }
        Some(out)
    }

    fn cell_of(&self, point: [f32; 2]) -> (usize, usize) {
        let col = ((point[0] - self.origin[0]) / CELL).max(0.0) as usize;
        let row = ((point[1] - self.origin[1]) / CELL).max(0.0) as usize;
        (col.min(self.cols - 1), row.min(self.rows - 1))
    }

    /// How many triangles the terrain has.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    /// The height of the terrain surface at `at` (x, y) nearest to `near`, among the surfaces from `below` under it to `above` over it.
    /// Two surfaces equally near: the lower. `None` when there is no terrain there in that window.
    pub fn surface(&self, at: [f32; 2], near: f32, below: f32, above: f32) -> Option<f32> {
        if at[0] < self.origin[0] - CELL
            || at[1] < self.origin[1] - CELL
            || at[0] > self.origin[0] + CELL * (self.cols as f32 + 1.0)
            || at[1] > self.origin[1] + CELL * (self.rows as f32 + 1.0)
        {
            return None;
        }
        let (col, row) = self.cell_of(at);
        let mut best: Option<(f32, f32)> = None;
        for &number in &self.cells[row * self.cols + col] {
            let Some(z) = height_in(&self.triangles[number as usize], at) else { continue };
            if z < near - below || z > near + above {
                continue;
            }
            let gap = (z - near).abs();
            // Worse than the best so far, or equally near and not lower: not taken.
            if best.is_some_and(|(known, low)| gap > known + TIE || (gap >= known - TIE && z >= low)) {
                continue;
            }
            best = Some((gap, z));
        }
        best.map(|(_, z)| z)
    }
}

/// The height of `triangle` at `at` in plan, `None` when the point is outside it or the triangle is edge-on.
fn height_in(triangle: &Triangle, at: [f32; 2]) -> Option<f32> {
    let [a, b, c] = *triangle;
    let (v0, v1, v2) = ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]], [at[0] - a[0], at[1] - a[1]]);
    let det = v0[0] * v1[1] - v1[0] * v0[1];
    if det.abs() < 1e-9 {
        return None;
    }
    let u = (v2[0] * v1[1] - v1[0] * v2[1]) / det;
    let v = (v0[0] * v2[1] - v2[0] * v0[1]) / det;
    if u < -EDGE_SLACK || v < -EDGE_SLACK || u + v > 1.0 + EDGE_SLACK {
        return None;
    }
    Some(a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]))
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
