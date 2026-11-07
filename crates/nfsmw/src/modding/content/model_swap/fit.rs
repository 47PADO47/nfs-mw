//! Fitting a replacement model into the bounds of the solid it replaces.
//!
//! A replacement keeps what makes the solid the game's own: its name (so the hash the scenery refers to it by), its flags (the
//! sway bit among them), its pivot matrix, and the polygon count and density its header carries (the scenery's choice of
//! level of detail reads them). What changes is the geometry: the model, scaled and placed within the old bounds. The solid is
//! made by writing the result with `blackbox_solid::write_geometry_pack` and reading it back, so it is a solid exactly as the
//! game's own loader would have made it (groups, vertex buffers and indices in the layout the renderer expects).
//!
//! Two kinds of solid are replaced ([`Frame`]): a **local** one is modelled around its own origin and the scenery places it with a
//! matrix; a **world** copy (`<NAME>_DEINST<n>_<tile>`) has that matrix baked into its vertices, so its bounds are where it
//! stands and a row of trees in it can run in any direction. The model is fitted to the first by its bounds and to the second by
//! the footprint of its vertices ([`Footprint`]).

use anyhow::{Context, Result, bail};
use blackbox_modding::content::SwapFit;
use blackbox_solid::{GroupSource, Solid, SolidSource, Vertex, read_solids, write_geometry_pack};

/// The smallest and largest scale a model is given to reach the replaced height.
pub const MIN_SCALE: f32 = 0.25;
pub const MAX_SCALE: f32 = 4.0;
/// A copy in a row is never scaled past this: a row of the full height of a tall solid would be a few huge trees.
pub const ROW_MAX_SCALE: f32 = 1.5;
/// The most copies of a model in a row.
pub const ROW_MAX_COPIES: usize = 8;
/// The share of its width a copy in a row takes along the row: the crowns overlap.
const ROW_OVERLAP: f32 = 0.8;
/// Each copy of a row is turned this many radians more than the last (the golden angle), so the copies are not alike.
const TURN: f32 = 2.399_963;

/// What space the replaced solid's vertices are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// Around the solid's own origin; the scenery places it with a matrix.
    Local,
    /// Where it stands in the world (a baked copy of an instance).
    World,
}

/// Where one copy of the model stands: a scale, a turn about the vertical axis, and where its base point goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub scale: f32,
    pub yaw: f32,
    pub offset: [f32; 3],
}

/// The ground plan of a solid in world coordinates: the middle of the oriented box that holds its vertices, the direction of
/// the box's long side in radians from +x, and the length of that side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Footprint {
    pub centre: [f32; 2],
    pub angle: f32,
    pub length: f32,
}

/// A model made ready to be fitted many times: its groups and its bounds, read once.
pub struct Prepared {
    source: SolidSource,
    low: [f32; 3],
    high: [f32; 3],
}

impl Prepared {
    /// Fails when the model cannot be written back (it has no groups, or wide vertices), or has no vertices or no height.
    pub fn new(model: &Solid) -> Result<Self> {
        let source = SolidSource::from_solid(model).with_context(|| format!("the model {}", model.name))?;
        let Some((low, high)) = vertex_bounds(&source.groups) else {
            bail!("the model {} has no vertices", model.name)
        };
        if high[2] <= 0.01 {
            bail!("the model {} has no height", model.name);
        }
        Ok(Self { source, low, high })
    }
}

/// The solid that takes the place of `stock`: `model`, fitted as `how` says in the space `frame` names. Fails when the solid
/// has no height or the result cannot be written.
pub fn fit_prepared(model: &Prepared, stock: &Solid, how: SwapFit, frame: Frame) -> Result<Solid> {
    let stock_height = stock.bounds_max[2] - stock.bounds_min[2];
    if stock_height <= 0.01 {
        bail!("the solid {} has no height", stock.name);
    }
    let bounds = (stock.bounds_min, stock.bounds_max);
    let placements = match frame {
        Frame::Local => placements(how, (model.low, model.high), bounds),
        Frame::World => world_placements(how, (model.low, model.high), bounds, &footprint(stock)),
    };
    let source = &model.source;
    let mut groups = Vec::with_capacity(source.groups.len() * placements.len());
    for at in &placements {
        for group in &source.groups {
            let vertices = group.vertices.iter().map(|vertex| place(vertex, at)).collect();
            groups.push(GroupSource { vertices, bounds: None, ..group.clone() });
        }
    }
    let mut out = SolidSource::new(stock.name.clone(), groups);
    out.flags = stock.flags;
    out.transform = stock.transform;
    out.density = Some(stock.density);
    out.polygon_count = Some(stock.num_polys);
    out.texture_hashes = source.texture_hashes.clone();
    out.light_material_hashes = source.light_material_hashes.clone();
    let bytes = write_geometry_pack(&[out]).with_context(|| format!("writing the replacement of {}", stock.name))?;
    let mut solids = read_solids(&bytes).with_context(|| format!("reading the replacement of {}", stock.name))?;
    solids.pop().with_context(|| format!("the replacement of {} holds no solid", stock.name))
}

/// The scale a model gets in a row to reach `wanted` times its height, and how many copies of it a row of `length` metres holds
/// when a copy takes `model_width` metres at that scale (less their overlap).
fn row_scale_and_count(wanted: f32, length: f32, model_width: f32) -> (f32, usize) {
    let scale = wanted.clamp(MIN_SCALE, ROW_MAX_SCALE);
    let width = model_width * scale * ROW_OVERLAP;
    let count = ((length / width.max(0.01)).round() as usize).clamp(1, ROW_MAX_COPIES);
    (scale, count)
}

/// The copies of a model with bounds `model` that fill a local solid with bounds `stock`.
pub fn placements(how: SwapFit, model: ([f32; 3], [f32; 3]), stock: ([f32; 3], [f32; 3])) -> Vec<Placement> {
    let (low, high) = model;
    let (stock_low, stock_high) = stock;
    let wanted = (stock_high[2] - stock_low[2]) / high[2];
    if how == SwapFit::Height {
        return vec![Placement {
            scale: wanted.clamp(MIN_SCALE, MAX_SCALE),
            yaw: 0.0,
            offset: [0.0, 0.0, stock_low[2]],
        }];
    }
    let (along_x, length) = match stock_high[0] - stock_low[0] >= stock_high[1] - stock_low[1] {
        true => (true, stock_high[0] - stock_low[0]),
        false => (false, stock_high[1] - stock_low[1]),
    };
    let (scale, count) = row_scale_and_count(wanted, length, (high[0] - low[0]).max(high[1] - low[1]));
    let across = |axis: usize| 0.5 * (stock_low[axis] + stock_high[axis]);
    (0..count)
        .map(|n| {
            let along_axis = usize::from(!along_x);
            let along = stock_low[along_axis] + (n as f32 + 0.5) * length / count as f32;
            let offset = match along_x {
                true => [along, across(1), stock_low[2]],
                false => [across(0), along, stock_low[2]],
            };
            Placement { scale, yaw: n as f32 * TURN, offset }
        })
        .collect()
}

/// The copies of a model with bounds `model` that fill a world copy with bounds `stock` and ground plan `plan`: one at the middle
/// of the plan, or a row along its long side, whichever way that points.
pub fn world_placements(
    how: SwapFit,
    model: ([f32; 3], [f32; 3]),
    stock: ([f32; 3], [f32; 3]),
    plan: &Footprint,
) -> Vec<Placement> {
    let (low, high) = model;
    let (stock_low, stock_high) = stock;
    let wanted = (stock_high[2] - stock_low[2]) / high[2];
    if how == SwapFit::Height {
        return vec![Placement {
            scale: wanted.clamp(MIN_SCALE, MAX_SCALE),
            yaw: 0.0,
            offset: [plan.centre[0], plan.centre[1], stock_low[2]],
        }];
    }
    let (scale, count) = row_scale_and_count(wanted, plan.length, (high[0] - low[0]).max(high[1] - low[1]));
    let (sin, cos) = plan.angle.sin_cos();
    (0..count)
        .map(|n| {
            let along = ((n as f32 + 0.5) / count as f32 - 0.5) * plan.length;
            let offset = [plan.centre[0] + cos * along, plan.centre[1] + sin * along, stock_low[2]];
            Placement { scale, yaw: n as f32 * TURN, offset }
        })
        .collect()
}

/// The ground plan of the vertices of `stock`: the long side's direction is the principal axis of their (x, y) positions, and
/// the centre and length are those of the box of that orientation that holds them all. A solid with no vertices has the middle
/// of its bounds and its longer side.
pub fn footprint(stock: &Solid) -> Footprint {
    let count = stock.vertices.len().max(1) as f32;
    let (mut mean_x, mut mean_y) = (0.0_f32, 0.0_f32);
    for vertex in &stock.vertices {
        mean_x += vertex.position[0];
        mean_y += vertex.position[1];
    }
    mean_x /= count;
    mean_y /= count;
    let (mut xx, mut xy, mut yy) = (0.0_f32, 0.0_f32, 0.0_f32);
    for vertex in &stock.vertices {
        let (dx, dy) = (vertex.position[0] - mean_x, vertex.position[1] - mean_y);
        xx += dx * dx;
        xy += dx * dy;
        yy += dy * dy;
    }
    let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
    let (sin, cos) = angle.sin_cos();
    let (mut along_low, mut along_high) = (f32::MAX, f32::MIN);
    let (mut across_low, mut across_high) = (f32::MAX, f32::MIN);
    for vertex in &stock.vertices {
        let (dx, dy) = (vertex.position[0] - mean_x, vertex.position[1] - mean_y);
        let (along, across) = (dx * cos + dy * sin, dy * cos - dx * sin);
        along_low = along_low.min(along);
        along_high = along_high.max(along);
        across_low = across_low.min(across);
        across_high = across_high.max(across);
    }
    if along_low > along_high {
        let (low, high) = (stock.bounds_min, stock.bounds_max);
        let centre = [0.5 * (low[0] + high[0]), 0.5 * (low[1] + high[1])];
        return Footprint { centre, angle: 0.0, length: (high[0] - low[0]).max(high[1] - low[1]) };
    }
    let (along_mid, across_mid) = (0.5 * (along_low + along_high), 0.5 * (across_low + across_high));
    Footprint {
        centre: [mean_x + cos * along_mid - sin * across_mid, mean_y + sin * along_mid + cos * across_mid],
        angle,
        length: along_high - along_low,
    }
}

/// A model vertex after the placement: scaled and turned about the vertical axis, and moved. The model's origin is its base, so
/// the origin goes to the placement's offset (a model's roots may reach below it). The normal is turned only (the scale is
/// even).
fn place(vertex: &Vertex, at: &Placement) -> Vertex {
    let (sin, cos) = at.yaw.sin_cos();
    let [x, y, z] = vertex.position;
    let (x, y) = (x * at.scale, y * at.scale);
    let [nx, ny, nz] = vertex.normal;
    Vertex {
        position: [cos * x - sin * y + at.offset[0], sin * x + cos * y + at.offset[1], z * at.scale + at.offset[2]],
        normal: [cos * nx - sin * ny, sin * nx + cos * ny, nz],
        ..*vertex
    }
}

/// The lowest and highest corner of the vertices of `groups`, `None` when there are none.
fn vertex_bounds(groups: &[GroupSource]) -> Option<([f32; 3], [f32; 3])> {
    let mut vertices = groups.iter().flat_map(|group| group.vertices.iter());
    let first = vertices.next()?.position;
    let (mut low, mut high) = (first, first);
    for vertex in vertices {
        for ((low, high), value) in low.iter_mut().zip(high.iter_mut()).zip(vertex.position) {
            *low = low.min(value);
            *high = high.max(value);
        }
    }
    Some((low, high))
}
