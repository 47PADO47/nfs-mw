//! A scenery section built in memory from placements (docs/specs/mod-world-content.md, section 2.4). A mod's objects have
//! no streamed `ScenerySection` chunk, so the game builds one here from the solids it has loaded, and nothing is written
//! to disk.

use crate::{SceneryInfo, SceneryInstance, ScenerySection};

/// The largest rotation component the format stores: `i16 / 8192`.
pub const MAX_COMPONENT: f32 = i16::MAX as f32 / 8192.0;

/// One object to place: the model it uses, its bounds, and where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// The model's name, for the info record.
    pub name: String,
    /// `bStringHash` of the solid for each LOD slot: 0 the player-view model, 1 the mirror model, 2 the coarse model, and
    /// 3 unused (0).
    pub solids: [u32; 4],
    /// The model's local box, metres, in model space.
    pub local_min: [f32; 3],
    pub local_max: [f32; 3],
    /// The model's bounding radius, metres.
    pub radius: f32,
    /// The position, metres.
    pub position: [f32; 3],
    /// The rows of the rotation: the object's x, y and z axes in world space, scale included.
    pub rows: [[f32; 3]; 3],
    /// The raw exclude flags of the instance.
    pub exclude_flags: u32,
}

/// Why a section cannot be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// A rotation component is not finite or does not fit the format. `placement` counts from 0.
    Rotation { placement: usize },
    /// A position is not finite.
    Position { placement: usize },
    /// More placements than the index can number.
    TooMany,
}

/// The section of `placements`, numbered `section_number`. Instances keep the order of `placements`, and each has its own
/// info record at the same index.
pub fn section(section_number: u32, placements: &[Placement]) -> Result<ScenerySection, BuildError> {
    if i16::try_from(placements.len()).is_err() {
        return Err(BuildError::TooMany);
    }
    let mut infos = Vec::with_capacity(placements.len());
    let mut instances = Vec::with_capacity(placements.len());
    for (at, placement) in placements.iter().enumerate() {
        check(at, placement)?;
        let (bbox_min, bbox_max) = bounds(placement);
        infos.push(SceneryInfo {
            name: placement.name.clone(),
            solid_keys: placement.solids.to_vec(),
            radius: placement.radius,
            hierarchy_hash: 0,
        });
        instances.push(SceneryInstance {
            bbox_min,
            bbox_max,
            exclude_flags: placement.exclude_flags,
            preculler_index: 0,
            lighting_context: 0,
            position: placement.position,
            rotation: placement.rows,
            info_index: at as i16,
        });
    }
    Ok(ScenerySection { section_number, infos, instances, override_hooks: Vec::new() })
}

/// The rows of a yaw: `degrees` about the up axis, anticlockwise from +x, scaled by `scale`. The rows are the object's
/// axes, in the row-vector convention of the scenery format.
pub fn yaw_rows(degrees: f32, scale: f32) -> [[f32; 3]; 3] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [[cos * scale, sin * scale, 0.0], [-sin * scale, cos * scale, 0.0], [0.0, 0.0, scale]]
}

fn check(at: usize, placement: &Placement) -> Result<(), BuildError> {
    if !placement.position.iter().all(|v| v.is_finite()) {
        return Err(BuildError::Position { placement: at });
    }
    let fits = placement.rows.iter().flatten().all(|v| v.is_finite() && v.abs() <= MAX_COMPONENT);
    if !fits {
        return Err(BuildError::Rotation { placement: at });
    }
    Ok(())
}

/// The world-space box of a placed model: the eight corners of its local box, turned by the rows and moved to the
/// position, and the min and max of them.
fn bounds(placement: &Placement) -> ([f32; 3], [f32; 3]) {
    let (lo, hi) = (placement.local_min, placement.local_max);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let [r0, r1, r2] = placement.rows;
    for corner in 0..8usize {
        let local = [0, 1, 2].map(|axis| [lo[axis], hi[axis]][(corner >> axis) & 1]);
        let world =
            [0, 1, 2].map(|col| local[0] * r0[col] + local[1] * r1[col] + local[2] * r2[col] + placement.position[col]);
        for (axis, value) in world.into_iter().enumerate() {
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    (min, max)
}
