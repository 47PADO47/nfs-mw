//! Where the copies of a model stand inside the solid they replace, and how big each is.
//!
//! The rule, in short (`docs/modding/effect-variants.md`, "Profile format of the trees"): a replacement **never grows beyond the
//! footprint it replaces**. A single tree is one copy; a stretch of trees (a row, or any world-space copy, which may be a whole
//! hillside of trees) gets one copy for each *stretch of the old geometry*, standing where that geometry stood, on **its**
//! floor (a solid modelled around its own origin stands on the origin, which is the ground). So a line of trees that climbs a slope or follows a road keeps its shape, and no tree floats over a lower floor or
//! sinks into a higher one. Each copy is held to the width of the geometry it replaces ([`FitLimits`]): it is squeezed sideways
//! if that is enough, made smaller if not, and when it would end up too short the solid keeps the game's own model.

use std::collections::BTreeMap;

use blackbox_modding::content::{FitLimits, SwapFit};
use blackbox_solid::Vertex;

/// The smallest and largest scale a single tree is given to reach the replaced height.
pub const MIN_SCALE: f32 = 0.25;
pub const MAX_SCALE: f32 = 4.0;
/// A copy in a row is never scaled past this: a row of the full height of a tall solid would be a few huge trees.
pub const ROW_MAX_SCALE: f32 = 1.5;
/// The most copies of a model in one solid.
pub const MAX_COPIES: usize = 32;
/// The most vertices the copies of one solid may have together: a writer keeps the groups of an effect in one 16-bit buffer.
pub const MAX_VERTICES: usize = 60_000;
/// The share of its width a copy in a row takes along the row: the crowns overlap.
const OVERLAP: f32 = 0.8;
/// The least width of a stretch (a cell of the plan), metres, and the least vertices it needs to hold a copy.
const MIN_CELL: f32 = 3.0;
const MIN_CELL_VERTICES: usize = 3;
/// A stretch is never taken thinner than this share of the middle stretch's thickness (its few edge vertices would give a stunted copy).
const MIN_BAND_SHARE: f32 = 0.5;
/// A solid no longer than this many crowns is one stretch: one copy, not a copy for each piece of a grid it happens to straddle.
const SINGLE_STRETCH: f32 = 1.5;
/// More than this share of the stretches that cannot be fitted leaves the solid its own model.
const MAX_FAILED_SHARE: f32 = 1.0 / 3.0;
/// Each copy of a row is turned this many radians more than the last (the golden angle), so the copies are not alike.
const TURN: f32 = 2.399_963;

/// What a model is, measured once: its origin is the foot of the trunk and z is up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measure {
    /// The longer side of its footprint (x or y), metres.
    pub width: f32,
    /// How far it reaches above its origin.
    pub height: f32,
    /// How many vertices it has.
    pub vertices: usize,
}

/// What the replaced solid is, as the placement needs it.
#[derive(Debug, Clone, Copy)]
pub struct Stock<'a> {
    pub vertices: &'a [Vertex],
    pub bounds: ([f32; 3], [f32; 3]),
}

/// Where one copy stands: a scale across (x and y) and one up (z), a turn about the vertical axis, and where its foot goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub across: f32,
    pub up: f32,
    pub yaw: f32,
    pub offset: [f32; 3],
}

/// The ground a single copy has to fill.
struct Target {
    centre: [f32; 2],
    floor: f32,
    top: f32,
    width: f32,
}

/// One copy for `target`, scaled to its height by at most `cap`, held to its width by `limits`. `None` when it would be shorter
/// than `limits.min_height` of the height wanted.
fn tree(model: &Measure, target: &Target, cap: f32, limits: &FitLimits) -> Option<Placement> {
    let up = ((target.top - target.floor) / model.height.max(0.1)).clamp(MIN_SCALE, cap);
    let sideways = limits.spread * target.width / model.width.max(0.01);
    let at = |across: f32, up: f32| Placement {
        across,
        up,
        yaw: 0.0,
        offset: [target.centre[0], target.centre[1], target.floor],
    };
    if sideways >= limits.squeeze * up {
        return Some(at(sideways.min(up), up));
    }
    let shrunk = sideways / limits.squeeze;
    if shrunk < limits.min_height * up {
        return None;
    }
    Some(at(sideways, shrunk))
}

/// The one copy of a model that replaces a solid modelled around its own origin (the scenery places it with a matrix): at the
/// origin, which is the ground the scenery puts it on (the old solid may be a crown that begins above it), as tall as the solid
/// and no wider than it. `None` when it cannot be fitted.
pub fn single(model: &Measure, stock: &Stock, limits: &FitLimits) -> Option<Placement> {
    let (low, high) = stock.bounds;
    let target =
        Target { centre: [0.0, 0.0], floor: 0.0, top: high[2], width: (high[0] - low[0]).max(high[1] - low[1]) };
    tree(model, &target, MAX_SCALE, limits)
}

/// A stretch of the old geometry: the vertices that fall in one cell of the plan.
#[derive(Clone, Copy)]
struct Cell {
    count: usize,
    low: [f32; 2],
    high: [f32; 2],
    floor: f32,
    top: f32,
}

/// The cells of side `size` that hold the vertices, in order, the grid starting at the lowest corner of `vertices`.
fn grid(vertices: &[Vertex], size: f32) -> Vec<Cell> {
    let corner = vertices.iter().fold([f32::MAX; 2], |at, v| [at[0].min(v.position[0]), at[1].min(v.position[1])]);
    let mut cells: BTreeMap<(i32, i32), Cell> = BTreeMap::new();
    for vertex in vertices {
        let [x, y, z] = vertex.position;
        let key = (((x - corner[0]) / size).floor() as i32, ((y - corner[1]) / size).floor() as i32);
        let cell = cells.entry(key).or_insert(Cell { count: 0, low: [x, y], high: [x, y], floor: z, top: z });
        cell.count += 1;
        cell.low = [cell.low[0].min(x), cell.low[1].min(y)];
        cell.high = [cell.high[0].max(x), cell.high[1].max(y)];
        cell.floor = cell.floor.min(z);
        cell.top = cell.top.max(z);
    }
    cells.into_values().filter(|cell| cell.count >= MIN_CELL_VERTICES).collect()
}

/// The copies of a model that replace a solid with `stock`'s geometry: one for each stretch of it (a cell of the plan as wide as a
/// copy), on that stretch's own floor, as thick as it. Empty when the solid keeps its own model: nothing to stand on, or too many
/// stretches that no copy can be fitted to (see [`FitLimits`]).
pub fn copies(model: &Measure, stock: &Stock, how: SwapFit, limits: &FitLimits) -> Vec<Placement> {
    let (low, high) = stock.bounds;
    let (cap, overlap) = match how {
        SwapFit::Height => (MAX_SCALE, 1.0),
        SwapFit::Row => (ROW_MAX_SCALE, OVERLAP),
    };
    let reference = ((high[2] - low[2]) / model.height.max(0.1)).clamp(MIN_SCALE, cap);
    let mut size = (model.width * reference * overlap).max(MIN_CELL);
    let extent = (high[0] - low[0]).max(high[1] - low[1]);
    if extent <= SINGLE_STRETCH * size {
        size = extent * 1.001 + 0.001;
    }
    let mut cells = grid(stock.vertices, size);
    for _ in 0..8 {
        if cells.len() <= MAX_COPIES && cells.len() * model.vertices <= MAX_VERTICES {
            break;
        }
        size *= 1.4;
        cells = grid(stock.vertices, size);
    }
    if cells.is_empty() || cells.len() > MAX_COPIES {
        return Vec::new();
    }
    let mut bands: Vec<f32> = cells.iter().map(|cell| cell.top - cell.floor).collect();
    bands.sort_by(f32::total_cmp);
    let middle = bands[bands.len() / 2];
    let mut placements = Vec::with_capacity(cells.len());
    for cell in &cells {
        let target = Target {
            centre: [0.5 * (cell.low[0] + cell.high[0]), 0.5 * (cell.low[1] + cell.high[1])],
            floor: cell.floor,
            top: cell.floor + (cell.top - cell.floor).max(MIN_BAND_SHARE * middle),
            width: (cell.high[0] - cell.low[0]).max(cell.high[1] - cell.low[1]).max(0.5 * size),
        };
        let Some(mut at) = tree(model, &target, cap, limits) else { continue };
        at.yaw = placements.len() as f32 * TURN;
        placements.push(at);
    }
    let failed = cells.len() - placements.len();
    if failed as f32 > MAX_FAILED_SHARE * cells.len() as f32 {
        return Vec::new();
    }
    placements
}

#[cfg(test)]
#[path = "place_tests.rs"]
mod tests;
