//! A spatial hash over the horizontal plane (x and z), for relevancy queries: which entities are within a radius.

use std::collections::HashMap;
use std::ops::RangeInclusive;

/// A grid cell: `(floor(x / size), floor(z / size))`.
type Cell = (i32, i32);
/// One stored point: `(id, x, z)`.
type Point = (u16, f32, f32);

/// Points bucketed into square cells of the horizontal plane. Clearing keeps every cell's capacity.
#[derive(Debug, Clone)]
pub struct SpatialHash {
    cell: f32,
    cells: HashMap<Cell, Vec<Point>>,
}

impl SpatialHash {
    /// An empty hash with square cells of `cell_size`. A size that is not finite and positive is replaced by 1.
    pub fn new(cell_size: f32) -> Self {
        let cell = match cell_size {
            size if size.is_finite() && size > 0.0 => size,
            _ => 1.0,
        };
        Self { cell, cells: HashMap::new() }
    }

    /// Removes every point. Keeps the cells and their capacity.
    pub fn clear(&mut self) {
        for points in self.cells.values_mut() {
            points.clear();
        }
    }

    /// Adds entity `id` at `(x, z)`.
    pub fn insert(&mut self, id: u16, x: f32, z: f32) {
        let key = (self.cell_of(x), self.cell_of(z));
        self.cells.entry(key).or_default().push((id, x, z));
    }

    /// Clears `out`, then pushes the ids within `radius` of `(x, z)`, sorted ascending without duplicates. The distance
    /// is checked exactly; a negative or NaN radius acts as 0.
    pub fn query(&self, x: f32, z: f32, radius: f32, out: &mut Vec<u16>) {
        out.clear();
        let r = radius.max(0.0);
        let (x0, x1) = (self.cell_of(x - r), self.cell_of(x + r));
        let (z0, z1) = (self.cell_of(z - r), self.cell_of(z + r));
        let width = i64::from(x1) - i64::from(x0) + 1;
        let depth = i64::from(z1) - i64::from(z0) + 1;
        let probe = Probe { x, z, r2: r * r };
        let window = (x0..=x1, z0..=z1);
        match width.saturating_mul(depth) > self.cells.len() as i64 {
            // The query covers more cells than the hash holds: walk the occupied cells instead.
            true => self.scan_occupied(&window, probe, out),
            false => self.scan_window(&window, probe, out),
        }
        out.sort_unstable();
        out.dedup();
    }

    /// Visits every cell of the window, looking each one up.
    fn scan_window(&self, window: &(RangeInclusive<i32>, RangeInclusive<i32>), probe: Probe, out: &mut Vec<u16>) {
        for cx in window.0.clone() {
            for cz in window.1.clone() {
                if let Some(points) = self.cells.get(&(cx, cz)) {
                    probe.collect(points, out);
                }
            }
        }
    }

    /// Visits the occupied cells and keeps those inside the window.
    fn scan_occupied(&self, window: &(RangeInclusive<i32>, RangeInclusive<i32>), probe: Probe, out: &mut Vec<u16>) {
        for (&(cx, cz), points) in &self.cells {
            if window.0.contains(&cx) && window.1.contains(&cz) {
                probe.collect(points, out);
            }
        }
    }

    /// The cell index of a coordinate.
    fn cell_of(&self, value: f32) -> i32 {
        (value / self.cell).floor() as i32
    }
}

/// A radius query: the centre and the squared radius.
#[derive(Debug, Clone, Copy)]
struct Probe {
    x: f32,
    z: f32,
    r2: f32,
}

impl Probe {
    /// Pushes the ids of `points` whose squared distance to the centre is at most the squared radius.
    fn collect(self, points: &[Point], out: &mut Vec<u16>) {
        for &(id, px, pz) in points {
            let dx = px - self.x;
            let dz = pz - self.z;
            if dx * dx + dz * dz <= self.r2 {
                out.push(id);
            }
        }
    }
}
