//! Building the collision bounds of a vehicle body from points sampled on it.
//!
//! [`fit_body`] turns the points of a body (its collision mesh, or failing that its visible mesh, plus whatever must
//! stay inside such as the tyres) into a [`BoundsSet`] with the tree the vehicle code reads
//! (`docs/formats/collision.md`, "Bounds"; [`BodyGeometry`](crate::BodyGeometry) selects from it):
//!
//! - node 0, the root **box**: the extents of the points. Its half dimensions are the size the rigid body is built
//!   from, and its pivot is the box centre in the vehicle's own space;
//! - a **lower body box** from the ground to a fixed height, as wide and as long as the body;
//! - a **cabin box** on top of it, over the part of the body that is high;
//! - four **corner spheres** at the front end (low and high on each side) and two **roof spheres** at the front top
//!   corners of the cabin, which round off what a wall or another car meets first;
//! - a **mesh-versus-ground** node whose point cloud is the mirrored side silhouette of the body ([`profile`]).
//!
//! The proportions come from the 86 stock sets of `GlobalB.lzc`, which share one template: lower box top 0.752 m,
//! front sphere radius 0.329 m, roof sphere radius 0.237 m, and a sphere placed at a fixed fraction of its radius
//! from the body's edges. The numbers below are those measurements; nothing is copied from any one car's set.
//!
//! Axes are the bounds space of vehicles: x across, y up, z along the body (forward is +z), metres, with the origin
//! at the vehicle's own origin (on the ground, between the wheels).

mod profile;
mod validate;

pub use validate::BodyLimits;

use crate::bytes::malformed;
use crate::math::{Vec3, add, sub};
use crate::{Bounds, BoundsSet, Result, bounds_flags as flags};

/// Height of the lower body box above the lowest point (metres): 0.752 in every stock set.
const LOWER_HEIGHT: f32 = 0.752;
/// The lower box is never taller than this fraction of the body.
const LOWER_MAX_FRACTION: f32 = 0.65;
/// Points higher than this fraction of the way from the top of the lower box to the roof belong to the cabin.
const CABIN_FLOOR_FRACTION: f32 = 0.4;
/// Gap between the lower box and the cabin box.
const CABIN_GAP: f32 = 0.02;
/// Front corner sphere radius in stock sets, and how it sits: centre inset from the side by `0.6 r`, from the nose by
/// `0.5 r`, low centre `0.55 r` above the lowest point, high centre `1.19 r` above the low one.
const FRONT_RADIUS: f32 = 0.329;
const FRONT_SIDE_INSET: f32 = 0.6;
const FRONT_NOSE_INSET: f32 = 0.5;
const FRONT_LOW: f32 = 0.55;
const FRONT_HIGH_STEP: f32 = 1.19;
/// Roof sphere radius in stock sets: centre inset from the cabin side by `0.5 r`, below the cabin top by `0.54 r`,
/// behind the cabin front by `0.55 r`.
const ROOF_RADIUS: f32 = 0.237;
const ROOF_SIDE_INSET: f32 = 0.5;
const ROOF_TOP_INSET: f32 = 0.54;
const ROOF_FRONT_INSET: f32 = 0.55;
/// A sphere is never wider than this fraction of the box it sits on.
const SPHERE_MAX_FRACTION: f32 = 0.4;
/// Silhouette corners per side, and how far (metres) a corner looks for the body width around it.
const SILHOUETTE_CORNERS: usize = 8;
const SILHOUETTE_REACH: f32 = 0.4;
/// Orientation of the mesh node: the half turn about `(1, -1, 0)` that stock sets store, which swaps its x and y.
const MESH_ORIENTATION: [f32; 4] = [std::f32::consts::FRAC_1_SQRT_2, -std::f32::consts::FRAC_1_SQRT_2, 0.0, 0.0];
/// Smallest extent that is a vehicle at all.
const MIN_SIZE: Vec3 = [0.5, 0.3, 1.0];

const ROOT: u16 = flags::BOX;
const BODY_BOX: u16 = flags::BOX | flags::PRIM_VS_WORLD | flags::PRIM_VS_OBJECTS;
const CORNER_SPHERE: u16 = flags::SPHERE | flags::PRIM_VS_WORLD;
const MESH: u16 = flags::BOX | flags::MESH_VS_GROUND;

/// What a fitted set is made with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitOptions {
    /// `simsurface` key of every node below the root (0 = the object's default).
    pub surface: u32,
    /// Height of the lower body box above the lowest point.
    pub lower_height: f32,
}

impl Default for FitOptions {
    fn default() -> Self {
        Self { surface: 0, lower_height: LOWER_HEIGHT }
    }
}

/// The set named `name_hash` for a body whose surface is sampled by `points` (vehicle axes, see the module docs).
/// Points that are not finite are ignored. Fails when too few points are left or they span less than a vehicle
/// ([`MIN_SIZE`] metres), and when a length does not fit the format.
pub fn fit_body(name_hash: u32, points: &[Vec3], options: &FitOptions) -> Result<BoundsSet> {
    let points: Vec<Vec3> = points.iter().copied().filter(|p| p.iter().all(|v| v.is_finite())).collect();
    if points.len() < 8 {
        return Err(malformed("body fit", format!("{} usable points", points.len())));
    }
    let (min, max) = extents(&points);
    let size = sub(max, min);
    if (0..3).any(|a| size[a] < MIN_SIZE[a]) {
        return Err(malformed("body fit", format!("a body of {size:?} metres is not a vehicle")));
    }
    let centre = [q((min[0] + max[0]) / 2.0), q((min[1] + max[1]) / 2.0), q((min[2] + max[2]) / 2.0)];
    let half = [q(size[0] / 2.0), q(size[1] / 2.0), q(size[2] / 2.0)];
    let mut nodes = vec![Bounds {
        orientation: [0.0, 0.0, 0.0, 1.0],
        position: [0.0; 3],
        flags: ROOT,
        half_dimensions: half,
        child_count: 0,
        point_cloud: None,
        pivot: centre,
        first_child: 1,
        radius: radius_of(half),
        surface: 0,
        name_hash: 0,
    }];
    let place = |flags: u16, at: Vec3, half: Vec3, radius: f32| {
        let position = [q(at[0] - centre[0]), q(at[1] - centre[1]), q(at[2] - centre[2])];
        Bounds {
            orientation: [0.0, 0.0, 0.0, 1.0],
            position,
            flags,
            half_dimensions: half,
            child_count: 0,
            point_cloud: None,
            pivot: [q(centre[0] + position[0]), q(centre[1] + position[1]), q(centre[2] + position[2])],
            first_child: -1,
            radius,
            surface: options.surface,
            name_hash: 0,
        }
    };
    // The lower body box: the full footprint up to the lower height.
    let lower_top = min[1] + options.lower_height.min(LOWER_MAX_FRACTION * size[1]);
    let lower_half = [half[0], q((lower_top - min[1]) / 2.0), half[2]];
    nodes.push(place(BODY_BOX, [centre[0], min[1] + lower_half[1], centre[2]], lower_half, radius_of(lower_half)));
    // The cabin box over the high part of the body.
    let cabin = cabin_box(&points, lower_top, max[1]);
    if let Some((cabin_min, cabin_max)) = cabin {
        let cabin_half = [
            q((cabin_max[0] - cabin_min[0]) / 2.0),
            q((cabin_max[1] - cabin_min[1]) / 2.0),
            q((cabin_max[2] - cabin_min[2]) / 2.0),
        ];
        let at = [
            (cabin_min[0] + cabin_max[0]) / 2.0,
            (cabin_min[1] + cabin_max[1]) / 2.0,
            (cabin_min[2] + cabin_max[2]) / 2.0,
        ];
        nodes.push(place(BODY_BOX, at, cabin_half, radius_of(cabin_half)));
    }
    // Front corner spheres, in the order the stock sets list them: right low, mesh, left low, right high, left high.
    let r = FRONT_RADIUS.min(SPHERE_MAX_FRACTION * half[0]);
    let front = |side: f32, high: bool| {
        let y = min[1] + FRONT_LOW * r + if high { FRONT_HIGH_STEP * r } else { 0.0 };
        let x = centre[0] + side * (half[0] - FRONT_SIDE_INSET * r);
        place(CORNER_SPHERE, [x, y, max[2] - FRONT_NOSE_INSET * r], [r; 3], r)
    };
    nodes.push(front(1.0, false));
    let mesh_at = nodes.len();
    nodes.push(front(-1.0, false));
    nodes.push(front(1.0, true));
    nodes.push(front(-1.0, true));
    // Roof spheres at the front top corners of the cabin.
    if let Some((cabin_min, cabin_max)) = cabin {
        let cabin_half_x = (cabin_max[0] - cabin_min[0]) / 2.0;
        let r = ROOF_RADIUS.min(SPHERE_MAX_FRACTION * cabin_half_x);
        let cabin_x = (cabin_min[0] + cabin_max[0]) / 2.0;
        for side in [1.0, -1.0] {
            let at = [
                cabin_x + side * (cabin_half_x - ROOF_SIDE_INSET * r),
                cabin_max[1] - ROOF_TOP_INSET * r,
                cabin_max[2] - ROOF_FRONT_INSET * r,
            ];
            nodes.push(place(CORNER_SPHERE, at, [r; 3], r));
        }
    }
    // The mesh-versus-ground node goes after the first front sphere, as in the stock sets; its cloud is relative to
    // the root pivot like every other position.
    let cloud: Vec<Vec3> = profile::silhouette(&points, centre[0], SILHOUETTE_CORNERS, SILHOUETTE_REACH)
        .into_iter()
        .map(|p| sub(p, centre))
        .collect();
    let (cloud_min, cloud_max) = extents(&cloud);
    let cloud_half =
        [(cloud_max[0] - cloud_min[0]) / 2.0, (cloud_max[1] - cloud_min[1]) / 2.0, (cloud_max[2] - cloud_min[2]) / 2.0];
    let mesh_centre = add(
        centre,
        [(cloud_min[0] + cloud_max[0]) / 2.0, (cloud_min[1] + cloud_max[1]) / 2.0, (cloud_min[2] + cloud_max[2]) / 2.0],
    );
    // The node's own box is stored in its swapped frame: x is the height, y the width.
    let mesh_half = [q(cloud_half[1]), q(cloud_half[0]), q(cloud_half[2])];
    let mut mesh = place(MESH, mesh_centre, mesh_half, radius_of(mesh_half));
    mesh.orientation = MESH_ORIENTATION;
    mesh.point_cloud = Some(0);
    nodes.insert(mesh_at, mesh);
    nodes[0].child_count = u8::try_from(nodes.len() - 1).map_err(|_| malformed("body fit", "too many nodes"))?;
    Ok(BoundsSet { name_hash, nodes, point_clouds: vec![cloud] })
}

/// The box over the high part of the body, or `None` when the body has none worth a box.
fn cabin_box(points: &[Vec3], lower_top: f32, top: f32) -> Option<(Vec3, Vec3)> {
    let floor = lower_top + CABIN_FLOOR_FRACTION * (top - lower_top);
    let high: Vec<Vec3> = points.iter().copied().filter(|p| p[1] >= floor).collect();
    if high.len() < 3 {
        return None;
    }
    let (min, max) = extents(&high);
    let bottom = lower_top + CABIN_GAP;
    let cabin = ([min[0], bottom, min[2]], [max[0], top, max[2]]);
    let size = sub(cabin.1, cabin.0);
    (size[0] >= 0.2 && size[1] >= 0.2 && size[2] >= 0.2).then_some(cabin)
}

fn extents(points: &[Vec3]) -> (Vec3, Vec3) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for p in points {
        for a in 0..3 {
            min[a] = min[a].min(p[a]);
            max[a] = max[a].max(p[a]);
        }
    }
    (min, max)
}

fn radius_of(half: Vec3) -> f32 {
    half[0].max(half[1]).max(half[2])
}

/// `v` rounded to the millimetre the format stores.
fn q(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests;
