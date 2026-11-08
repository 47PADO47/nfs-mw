//! Fitting a replacement model into the bounds of the solid it replaces.
//!
//! A replacement keeps what makes the solid the game's own: its name (so the hash the scenery refers to it by), its flags (the
//! sway bit among them), its pivot matrix, and the polygon count and density its header carries (the scenery's choice of
//! level of detail reads them). What changes is the geometry: copies of the model, placed where the old geometry stood by
//! [`place`](super::place) (which never grows a copy beyond the footprint it replaces). The solid is made by writing the result
//! with `blackbox_solid::write_geometry_pack` and reading it back, so it is a solid exactly as the game's own loader would
//! have made it (groups, vertex buffers and indices in the layout the renderer expects).
//!
//! Two kinds of solid are replaced ([`Frame`]): a **local** one is modelled around its own origin and the scenery places it with a
//! matrix; a **world** copy (`<NAME>_DEINST<n>_<tile>`) has that matrix baked into its vertices, so it stands where it is and
//! may be a whole stretch of trees.

use anyhow::{Context, Result, bail};
use blackbox_modding::content::{FitLimits, SwapFit};
use blackbox_solid::{GroupSource, Solid, SolidSource, Vertex, read_solids, write_geometry_pack};

use super::place::{self, Measure, Placement, Stock};

/// The effect of the game's leaf cards (the tree shader).
const LEAVES: u32 = 6;
/// The effects of a trunk (the world shader, and its normal-mapped kind).
const TRUNKS: [u32; 2] = [0, 3];

/// What space the replaced solid's vertices are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// Around the solid's own origin; the scenery places it with a matrix.
    Local,
    /// Where it stands in the world (a baked copy of an instance). `mirrored`: the instance it was made of is a mirror image, so the
    /// copy's triangles are wound the other way and the scenery draws it flipped back (`docs/specs/backface-culling.md`); the
    /// replacement is wound the same way, or its trunk would show its inside.
    World { mirrored: bool },
}

/// A model made ready to be fitted many times: its groups and what it measures, read once.
pub struct Prepared {
    source: SolidSource,
    measure: Measure,
}

impl Prepared {
    /// Fails when the model cannot be written back (it has no groups, or wide vertices), or has no vertices or no height.
    pub fn new(model: &Solid) -> Result<Self> {
        let source = SolidSource::from_solid(model).with_context(|| format!("the model {}", model.name))?;
        let Some(measure) = measure(&source.groups) else { bail!("the model {} has no vertices", model.name) };
        if measure.height <= 0.01 {
            bail!("the model {} has no height", model.name);
        }
        Ok(Self { source, measure })
    }
}

/// The solid that takes the place of `stock`: copies of `model`, fitted as `how` says in the space `frame` names and held to
/// `limits`. `None` when the solid keeps its own model: no copy can be fitted into its footprint within the limits. Fails when
/// the solid has no height or the result cannot be written.
pub fn fit_prepared(
    model: &Prepared,
    stock: &Solid,
    how: SwapFit,
    frame: Frame,
    limits: &FitLimits,
) -> Result<Option<Solid>> {
    if stock.bounds_max[2] - stock.bounds_min[2] <= 0.01 {
        bail!("the solid {} has no height", stock.name);
    }
    let seen = Stock {
        vertices: &stock.vertices,
        bounds: (stock.bounds_min, stock.bounds_max),
        trunk: stock.groups.iter().any(|group| TRUNKS.contains(&group.effect_id)),
    };
    let placements = match (frame, how) {
        (Frame::Local, SwapFit::Height) => place::single(&model.measure, &seen, limits).into_iter().collect(),
        _ => place::copies(&model.measure, &seen, how, limits),
    };
    if placements.is_empty() {
        return Ok(None);
    }
    let mirrored = matches!(frame, Frame::World { mirrored: true });
    let source = &model.source;
    let mut groups = Vec::with_capacity(source.groups.len() * placements.len());
    for at in &placements {
        for group in &source.groups {
            let vertices = group.vertices.iter().map(|vertex| place_vertex(vertex, at)).collect();
            let mut placed = GroupSource { vertices, bounds: None, ..group.clone() };
            if mirrored {
                placed.indices.as_chunks_mut::<3>().0.iter_mut().for_each(|triangle| triangle.swap(1, 2));
            }
            groups.push(placed);
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
    let solid = solids.pop().with_context(|| format!("the replacement of {} holds no solid", stock.name))?;
    Ok(Some(solid))
}

/// A model vertex after the placement: scaled (across in x and y, up in z), turned about the vertical axis, and moved. The model's
/// origin is its base, so the origin goes to the placement's offset (a model's roots may reach below it). The normal follows the
/// squeeze: it is divided by the scale of each axis, then turned and made a unit vector again.
fn place_vertex(vertex: &Vertex, at: &Placement) -> Vertex {
    let (sin, cos) = at.yaw.sin_cos();
    let [x, y, z] = vertex.position;
    let (x, y) = (x * at.across, y * at.across);
    let [nx, ny, nz] = vertex.normal;
    let (nx, ny, nz) = (nx / at.across, ny / at.across, nz / at.up);
    let length = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
    Vertex {
        position: [cos * x - sin * y + at.offset[0], sin * x + cos * y + at.offset[1], z * at.up + at.offset[2]],
        normal: [(cos * nx - sin * ny) / length, (sin * nx + cos * ny) / length, nz / length],
        ..*vertex
    }
}

/// What the groups measure: the footprint, the height, where the leaves begin (the lowest point of the model when it has no
/// leaves) and how many vertices. `None` with no vertices.
fn measure(groups: &[GroupSource]) -> Option<Measure> {
    let mut vertices = groups.iter().flat_map(|group| group.vertices.iter());
    let first = vertices.next()?.position;
    let (mut low, mut high) = (first, first);
    let mut count = 1;
    for vertex in vertices {
        count += 1;
        for ((low, high), value) in low.iter_mut().zip(high.iter_mut()).zip(vertex.position) {
            *low = low.min(value);
            *high = high.max(value);
        }
    }
    let leaves = groups.iter().filter(|group| group.effect_id == LEAVES).flat_map(|group| group.vertices.iter());
    let foliage_low = leaves.map(|vertex| vertex.position[2]).reduce(f32::min).unwrap_or(low[2]);
    Some(Measure { width: (high[0] - low[0]).max(high[1] - low[1]), height: high[2], foliage_low, vertices: count })
}
