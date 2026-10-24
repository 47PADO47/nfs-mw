//! A uniform grid over the map's roads, for culling: each cell of [`CELL_M`] metres lists the roads that may cross it,
//! so the roads round the car are found by reading the few cells under a circle, not by walking every road.
//!
//! A road is listed in every cell its pieces' bounding boxes cover. That is a little generous for a diagonal piece, and
//! never misses a cell the piece crosses.

use super::MapRoad;

/// The side of a grid cell, metres.
pub const CELL_M: f32 = 128.0;

/// The grid: its corner (the least x and y of every road point), its size in cells, and each cell's road ids.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoadGrid {
    origin: [f32; 2],
    columns: usize,
    rows: usize,
    /// Row by row; a road's id is its index in the road list.
    cells: Vec<Vec<u32>>,
}

impl RoadGrid {
    /// The grid over `roads`. Without a point it is empty, and every query finds nothing.
    pub fn new(roads: &[MapRoad]) -> Self {
        let mut min = [f32::INFINITY; 2];
        let mut max = [f32::NEG_INFINITY; 2];
        for point in roads.iter().flat_map(|road| &road.points) {
            min = [min[0].min(point[0]), min[1].min(point[1])];
            max = [max[0].max(point[0]), max[1].max(point[1])];
        }
        if !min[0].is_finite() || !min[1].is_finite() || !max[0].is_finite() || !max[1].is_finite() {
            return Self::default();
        }
        let columns = ((max[0] - min[0]) / CELL_M).floor() as usize + 1;
        let rows = ((max[1] - min[1]) / CELL_M).floor() as usize + 1;
        let mut grid = Self { origin: min, columns, rows, cells: vec![Vec::new(); columns * rows] };
        for (id, road) in roads.iter().enumerate() {
            let Ok(id) = u32::try_from(id) else { break };
            for piece in road.points.windows(2) {
                grid.add_piece(id, piece[0], piece[1]);
            }
        }
        grid
    }

    /// Lists road `id` in every cell the box of the piece from `a` to `b` covers.
    fn add_piece(&mut self, id: u32, a: [f32; 2], b: [f32; 2]) {
        let low = self.cell_of([a[0].min(b[0]), a[1].min(b[1])]);
        let high = self.cell_of([a[0].max(b[0]), a[1].max(b[1])]);
        for row in low.1..=high.1 {
            for column in low.0..=high.0 {
                let cell = &mut self.cells[row * self.columns + column];
                if cell.last() != Some(&id) {
                    cell.push(id);
                }
            }
        }
    }

    /// The cell holding `p`, clamped to the grid.
    fn cell_of(&self, p: [f32; 2]) -> (usize, usize) {
        let column = ((p[0] - self.origin[0]) / CELL_M).floor().max(0.0) as usize;
        let row = ((p[1] - self.origin[1]) / CELL_M).floor().max(0.0) as usize;
        (column.min(self.columns.saturating_sub(1)), row.min(self.rows.saturating_sub(1)))
    }

    /// The ids of the roads listed in the cells the box round a circle of `radius` round `centre` covers, sorted, each
    /// once. Nothing for a circle wholly outside the grid or a radius that is not a number of zero or more.
    pub fn query(&self, centre: [f32; 2], radius: f32) -> Vec<u32> {
        if self.cells.is_empty() || radius.is_nan() || radius < 0.0 {
            return Vec::new();
        }
        let (width, height) = (self.columns as f32 * CELL_M, self.rows as f32 * CELL_M);
        let (x, y) = (centre[0] - self.origin[0], centre[1] - self.origin[1]);
        if x + radius < 0.0 || y + radius < 0.0 || x - radius > width || y - radius > height {
            return Vec::new();
        }
        let low = self.cell_of([centre[0] - radius, centre[1] - radius]);
        let high = self.cell_of([centre[0] + radius, centre[1] + radius]);
        let mut found = Vec::new();
        for row in low.1..=high.1 {
            for column in low.0..=high.0 {
                found.extend_from_slice(&self.cells[row * self.columns + column]);
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// The grid's size in cells (columns, rows).
    #[cfg(test)]
    pub fn size(&self) -> (usize, usize) {
        (self.columns, self.rows)
    }
}
