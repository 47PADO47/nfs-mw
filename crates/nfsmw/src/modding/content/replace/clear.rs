//! Keeping the copies of a replacement off the objects next to them (`[fit] clearance`, `docs/modding/api-replace.md`, "Clearance").
//!
//! A replacement is fitted into the footprint of the solid it replaces, and a set may let it be wider than that footprint (`spread`): the
//! 2012 bushes are 1.5 times as wide as the 2005 ones. Next to a guard rail, a concrete barrier, a wall or a building that is too much: the
//! grass grows through it. So after a set's copies are placed, each is compared with the **solid objects of the same section** and
//! made smaller until it clears them: its scale across goes down in [`STEPS`] steps, from the placed width to the width of the stretch of
//! geometry it replaces ([`Placement::base`]), and never below that or below what `squeeze` and `min_height` allow. A copy that cannot
//! clear them at that width keeps it (the old bushes stood there too) unless its foot is inside an object: that copy is left out.
//!
//! What is an obstacle: the scenery instances of the section whose object is a wall, guard rail or barrier (`XW_`) or a building (`XB_`)
//! ([`is_obstacle`]); the hedges (`XW_HEDGE`) are growth, not objects. An instance is the oriented box of its solid's bounds when the
//! solid is known (the registry [`note_solids`] fills from every section a loader thread parses: most rails live in shared sections), else
//! the world box the instance carries. The test is in plan (two convex quadrilaterals, the separating axis rule) and in height (the copy's
//! range against the object's). Done once for a section, when the loader thread rewrites it: the section's obstacles are listed once and
//! each solid looks only at those near it.

use std::collections::HashMap;
use std::sync::RwLock;

use blackbox_modding::content::FitLimits;
use blackbox_solid::Solid;
use nfsmw_data::world::SectionData;

use super::place::{Measure, Placement};

/// The names (upper case) of the solids that are objects a copy must keep clear of: walls, guard rails, barriers, buildings.
pub const OBSTACLE_PREFIXES: &[&str] = &["XW_", "XB_"];
/// Among them, the ones that are growth (`XW_HEDGESTADIUM`), which the sets replace themselves.
const GROWTH_WORD: &str = "HEDGE";
/// How many sizes between the placed one and the smallest are tried.
pub const STEPS: usize = 4;
/// A copy and an object are at different heights when they are this far apart, metres.
const Z_MARGIN: f32 = 0.1;

/// One matrix, as the scenery has it (`SceneryInstance::matrix_columns`): `world = columns[0] * x + columns[1] * y + columns[2] * z + columns[3]`.
type Matrix = [[f32; 4]; 4];
type Quad = [[f32; 2]; 4];

const IDENTITY: Matrix = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// The bounds of the objects that were seen in sections, by the name hash of their solid: a shared section's rail is found by the tiles
/// that place it, whichever loaded first (a tile that loads before it uses the world box of the instance instead).
static SEEN: RwLock<Option<HashMap<u32, ([f32; 3], [f32; 3])>>> = RwLock::new(None);

/// What clearing did, summed over solids.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Copies looked at.
    pub copies: usize,
    /// Copies made smaller to clear their neighbours.
    pub shrunk: usize,
    /// Of them, the copies that still touch one at the smallest size (the width of the stretch they replace).
    pub held: usize,
    /// Copies left out because their foot is inside an object.
    pub skipped: usize,
}

impl Tally {
    pub fn merge(&mut self, other: Tally) {
        self.copies += other.copies;
        self.shrunk += other.shrunk;
        self.held += other.held;
        self.skipped += other.skipped;
    }
}

/// Whether a solid of this name is an object copies keep clear of.
pub fn is_obstacle(name: &str) -> bool {
    let named = |prefix: &str| name.get(..prefix.len()).is_some_and(|start| start.eq_ignore_ascii_case(prefix));
    if !OBSTACLE_PREFIXES.iter().any(|prefix| named(prefix)) {
        return false;
    }
    !name.to_ascii_uppercase().contains(GROWTH_WORD)
}

/// Remember the bounds of the obstacle solids of a section, for the tiles that place them (`SEEN`).
pub fn note_solids(solids: &[Solid]) {
    let new: Vec<&Solid> = solids.iter().filter(|solid| is_obstacle(&solid.name)).collect();
    if new.is_empty() {
        return;
    }
    let Ok(mut seen) = SEEN.write() else { return };
    let seen = seen.get_or_insert_with(HashMap::new);
    for solid in new {
        seen.insert(solid.name_hash, (solid.bounds_min, solid.bounds_max));
    }
}

/// The bounds of an obstacle solid seen so far.
fn seen(hash: u32) -> Option<([f32; 3], [f32; 3])> {
    SEEN.read().ok()?.as_ref()?.get(&hash).copied()
}

/// An object next to the copies: its plan and the heights it spans.
#[derive(Debug, Clone, Copy)]
pub struct Wall {
    pub quad: Quad,
    pub low: f32,
    pub high: f32,
}

/// One place of a solid the section places: its matrix and the box of the instance.
#[derive(Debug, Clone, Copy)]
struct Placed {
    matrix: Matrix,
    low: [f32; 3],
    high: [f32; 3],
}

/// The objects of one section and where it places the solids that are replaced, listed once.
#[derive(Debug, Default)]
pub struct Neighbours {
    walls: Vec<Wall>,
    placed: HashMap<u32, Vec<Placed>>,
}

fn transform(matrix: &Matrix, point: [f32; 3]) -> [f32; 3] {
    [0, 1, 2]
        .map(|row| matrix[0][row] * point[0] + matrix[1][row] * point[1] + matrix[2][row] * point[2] + matrix[3][row])
}

fn plan_of(low: [f32; 2], high: [f32; 2]) -> [[f32; 2]; 4] {
    [[low[0], low[1]], [high[0], low[1]], [high[0], high[1]], [low[0], high[1]]]
}

/// The extent of a quad: its lowest and highest corner.
fn extent(quad: &Quad) -> ([f32; 2], [f32; 2]) {
    let mut low = quad[0];
    let mut high = quad[0];
    for corner in quad {
        low = [low[0].min(corner[0]), low[1].min(corner[1])];
        high = [high[0].max(corner[0]), high[1].max(corner[1])];
    }
    (low, high)
}

impl Neighbours {
    /// Nothing to keep clear of.
    pub fn none() -> Self {
        Self::default()
    }

    /// Whether the section has no object a copy could touch.
    pub fn is_empty(&self) -> bool {
        self.walls.is_empty()
    }

    /// The objects of a section and the places of the solids `replaced` says are replaced. Called once for each section.
    pub fn of_section(data: &SectionData, replaced: impl Fn(u32) -> bool) -> Self {
        let mut out = Self::none();
        let known: HashMap<u32, &Solid> = data.solids.iter().map(|solid| (solid.name_hash, solid)).collect();
        for section in &data.scenery {
            for instance in &section.instances {
                let Some(info) = section.info_of(instance) else { continue };
                let matrix = instance.matrix_columns();
                let mut is_replaced = false;
                for &key in info.solid_keys.iter().filter(|key| **key != 0) {
                    if !replaced(key) {
                        continue;
                    }
                    is_replaced = true;
                    let at = Placed { matrix, low: instance.bbox_min, high: instance.bbox_max };
                    out.placed.entry(key).or_default().push(at);
                }
                if is_replaced || !is_obstacle(&info.name) {
                    continue;
                }
                let bounds = info.best_solid().and_then(|key| {
                    known.get(&key).map(|solid| (solid.bounds_min, solid.bounds_max)).or_else(|| seen(key))
                });
                out.walls.push(wall_of(&matrix, bounds, instance.bbox_min, instance.bbox_max));
            }
        }
        out
    }

    /// The same with the objects given: for tests.
    #[cfg(test)]
    pub fn from_parts(walls: Vec<Wall>, matrices: &[(u32, Matrix, [f32; 3], [f32; 3])]) -> Self {
        let mut out = Self { walls, placed: HashMap::new() };
        for &(hash, matrix, low, high) in matrices {
            out.placed.entry(hash).or_default().push(Placed { matrix, low, high });
        }
        out
    }

    /// Make the copies in `placements` keep clear of the objects: `solid` is the replaced solid when it is modelled around its own origin
    /// (the copy is clear when it is at every place the section puts it), `None` for a world-space copy standing where it is, whose box
    /// is `bounds`. A copy that must go is removed.
    pub fn clear(
        &self,
        solid: Option<u32>,
        bounds: ([f32; 3], [f32; 3]),
        model: &Measure,
        placements: &mut Vec<Placement>,
        limits: &FitLimits,
    ) -> Tally {
        let mut tally = Tally::default();
        let identity = [Placed { matrix: IDENTITY, low: bounds.0, high: bounds.1 }];
        let places: &[Placed] = match solid {
            None => &identity,
            Some(hash) => self.placed.get(&hash).map_or(&[][..], Vec::as_slice),
        };
        if places.is_empty() || self.walls.is_empty() {
            return tally;
        }
        let reach = model.width * placements.iter().map(|at| at.across).fold(0.0, f32::max);
        let near = self.near(places, reach);
        if near.is_empty() {
            return tally;
        }
        placements.retain_mut(|at| {
            tally.copies += 1;
            let Some(done) = clear_one(&near, places, model, at, limits) else {
                tally.skipped += 1;
                return false;
            };
            tally.shrunk += usize::from(done.shrunk);
            tally.held += usize::from(done.held);
            *at = done.at;
            true
        });
        tally
    }

    /// The objects whose box is within `reach` of one of the places.
    fn near(&self, places: &[Placed], reach: f32) -> Vec<Wall> {
        let close = |wall: &Wall| {
            let (low, high) = extent(&wall.quad);
            places.iter().any(|place| {
                low[0] <= place.high[0] + reach
                    && high[0] >= place.low[0] - reach
                    && low[1] <= place.high[1] + reach
                    && high[1] >= place.low[1] - reach
            })
        };
        self.walls.iter().copied().filter(close).collect()
    }
}

/// An obstacle's plan and heights: the oriented box of the solid's bounds as the instance places it, or the instance's own world box when
/// the solid is not known or the two disagree (the bounds of the solid must land inside the instance's box, a metre of slack).
fn wall_of(matrix: &Matrix, bounds: Option<([f32; 3], [f32; 3])>, low: [f32; 3], high: [f32; 3]) -> Wall {
    let boxed = Wall { quad: plan_of([low[0], low[1]], [high[0], high[1]]), low: low[2], high: high[2] };
    let Some((bmin, bmax)) = bounds else { return boxed };
    let corner = |x: f32, y: f32| {
        let at = transform(matrix, [x, y, bmin[2]]);
        [at[0], at[1]]
    };
    let quad = [corner(bmin[0], bmin[1]), corner(bmax[0], bmin[1]), corner(bmax[0], bmax[1]), corner(bmin[0], bmax[1])];
    let (qlow, qhigh) = extent(&quad);
    let within = (0..2).all(|axis| qlow[axis] >= low[axis] - 1.0 && qhigh[axis] <= high[axis] + 1.0);
    if !within {
        return boxed;
    }
    Wall { quad, ..boxed }
}

/// What became of one copy.
struct Done {
    at: Placement,
    shrunk: bool,
    held: bool,
}

/// The copy `at` made as wide as it can be while clearing `walls` at every one of `places`; `None` when it has to go.
fn clear_one(walls: &[Wall], places: &[Placed], model: &Measure, at: &Placement, limits: &FitLimits) -> Option<Done> {
    let smallest = smallest_scale(at, limits);
    let sizes = (0..=STEPS).map(|step| match smallest >= 1.0 {
        true => 1.0,
        false => smallest.powf(step as f32 / STEPS as f32),
    });
    for factor in sizes {
        let shrunk = scaled(at, factor, limits);
        if places.iter().all(|place| clear_at(walls, place, model, &shrunk)) {
            return Some(Done { at: shrunk, shrunk: factor < 1.0, held: false });
        }
        if smallest >= 1.0 {
            break;
        }
    }
    let shrunk = scaled(at, smallest, limits);
    let buried = places.iter().any(|place| walls.iter().any(|wall| contains(wall, place, &shrunk)));
    if buried {
        return None;
    }
    Some(Done { at: shrunk, shrunk: smallest < 1.0, held: true })
}

/// The least share of its width a copy is made, from the stretch it replaces ([`Placement::base`]) and from what `squeeze` and
/// `min_height` allow: made narrower, its height follows at `across / squeeze`, which may not fall below `min_height` of what it had.
fn smallest_scale(at: &Placement, limits: &FitLimits) -> f32 {
    let from_base = (at.base / at.across.max(1e-6)).clamp(0.0, 1.0);
    let from_height = limits.min_height * at.up * limits.squeeze / at.across.max(1e-6);
    from_base.max(from_height).min(1.0)
}

/// `at` with its width scaled by `factor`; its height follows when the squeeze limit needs it.
fn scaled(at: &Placement, factor: f32, limits: &FitLimits) -> Placement {
    let across = at.across * factor;
    Placement { across, up: at.up.min(across / limits.squeeze), ..*at }
}

/// The plan of the copy at one place, and the heights it spans there.
fn footprint(model: &Measure, at: &Placement, place: &Placed) -> (Quad, f32, f32) {
    let (sin, cos) = at.yaw.sin_cos();
    let corner = |x: f32, y: f32| {
        let (x, y) = (x * at.across, y * at.across);
        let local = [cos * x - sin * y + at.offset[0], sin * x + cos * y + at.offset[1], at.offset[2]];
        let world = transform(&place.matrix, local);
        [world[0], world[1]]
    };
    let (low, high) = (model.low, model.high);
    let quad = [corner(low[0], low[1]), corner(high[0], low[1]), corner(high[0], high[1]), corner(low[0], high[1])];
    let height = |z: f32| transform(&place.matrix, [at.offset[0], at.offset[1], at.offset[2] + z * at.up])[2];
    let (z0, z1) = (height(low[2]), height(high[2]));
    (quad, z0.min(z1), z0.max(z1))
}

/// Whether the copy `at` at `place` is clear of every wall.
fn clear_at(walls: &[Wall], place: &Placed, model: &Measure, at: &Placement) -> bool {
    let (quad, low, high) = footprint(model, at, place);
    walls.iter().all(|wall| !(wall.low <= high + Z_MARGIN && wall.high >= low - Z_MARGIN && overlap(&quad, &wall.quad)))
}

/// Whether the foot of the copy is inside the wall (at the same height).
fn contains(wall: &Wall, place: &Placed, at: &Placement) -> bool {
    let foot = transform(&place.matrix, at.offset);
    if foot[2] < wall.low - Z_MARGIN || foot[2] > wall.high + Z_MARGIN {
        return false;
    }
    let side = |from: [f32; 2], to: [f32; 2]| {
        (to[0] - from[0]) * (foot[1] - from[1]) - (to[1] - from[1]) * (foot[0] - from[0])
    };
    let signs: [f32; 4] = std::array::from_fn(|i| side(wall.quad[i], wall.quad[(i + 1) % 4]));
    signs.iter().all(|sign| *sign >= 0.0) || signs.iter().all(|sign| *sign <= 0.0)
}

/// Whether two convex quadrilaterals overlap (the separating axis rule on the edges of both).
fn overlap(a: &Quad, b: &Quad) -> bool {
    let project = |quad: &Quad, axis: [f32; 2]| {
        quad.iter().fold((f32::MAX, f32::MIN), |(low, high), p| {
            let along = p[0] * axis[0] + p[1] * axis[1];
            (low.min(along), high.max(along))
        })
    };
    for quad in [a, b] {
        for (from, to) in quad.iter().zip(quad.iter().cycle().skip(1)) {
            let axis = [from[1] - to[1], to[0] - from[0]];
            if axis == [0.0, 0.0] {
                continue;
            }
            let ((a_low, a_high), (b_low, b_high)) = (project(a, axis), project(b, axis));
            if a_high < b_low || b_high < a_low {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
#[path = "clear_tests.rs"]
mod tests;
