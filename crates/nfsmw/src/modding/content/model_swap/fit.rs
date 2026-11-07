//! Fitting a replacement model into the bounds of the solid it replaces.
//!
//! A replacement keeps what makes the solid the game's own: its name (so the hash the scenery refers to it by), its flags (the
//! sway bit among them), its pivot matrix, and the polygon count and density its header carries (the scenery's choice of
//! level of detail reads them). What changes is the geometry: the model, scaled and placed within the old bounds. The solid is
//! made by writing the result with `blackbox_solid::write_geometry_pack` and reading it back, so it is a solid exactly as the
//! game's own loader would have made it (groups, vertex buffers and indices in the layout the renderer expects).

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

/// Where one copy of the model stands: a scale, a turn about the vertical axis, and where its base point goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub scale: f32,
    pub yaw: f32,
    pub offset: [f32; 3],
}

/// The solid that takes the place of `stock`: `model`, fitted as `how` says. Fails when the model cannot be written back (it
/// has no groups, or wide vertices) or when either solid has no height.
pub fn fit(model: &Solid, stock: &Solid, how: SwapFit) -> Result<Solid> {
    let source = SolidSource::from_solid(model).with_context(|| format!("the model {}", model.name))?;
    let Some((low, high)) = vertex_bounds(&source.groups) else { bail!("the model {} has no vertices", model.name) };
    let model_height = high[2];
    let stock_height = stock.bounds_max[2] - stock.bounds_min[2];
    let has_height = |height: f32| height > 0.01;
    if !has_height(model_height) || !has_height(stock_height) {
        bail!("the model {} or the solid {} has no height", model.name, stock.name);
    }
    let placements = placements(how, (low, high), (stock.bounds_min, stock.bounds_max));
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

/// The copies of a model with bounds `model` that fill a solid with bounds `stock`.
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
    let scale = wanted.clamp(MIN_SCALE, ROW_MAX_SCALE);
    let (along_x, length) = match stock_high[0] - stock_low[0] >= stock_high[1] - stock_low[1] {
        true => (true, stock_high[0] - stock_low[0]),
        false => (false, stock_high[1] - stock_low[1]),
    };
    let width = (high[0] - low[0]).max(high[1] - low[1]) * scale * ROW_OVERLAP;
    let count = ((length / width.max(0.01)).round() as usize).clamp(1, ROW_MAX_COPIES);
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
