//! The collision bounds of a converted car: `files/CARS/<ID>/BOUNDS.BIN`, a `BoundsPack` with the car's own set.
//!
//! The 2012 game keeps a car's collision as a poly soup of a few hundred triangles (`docs/formats/chameleon-collision.md`),
//! with the body's real extents. The set is fitted to the points of that mesh, and of the tyres, which stand beyond the
//! body at the hubs, so the box the physics builds the rigid body from is as wide, long and tall as the car is. A car
//! whose bundle has no collision mesh (one of the 93) is fitted to its visible body mesh instead.
//!
//! The points are carried into the same car space as the body ([`build`] takes the transform the geometry uses), so
//! the bounds follow the model wherever the conversion puts it. The set is keyed by the car's folder name, the key the
//! game looks cars up by (`docs/modding/car-collision.md`).

use anyhow::{Context, Result};
use blackbox_attrib::vlt_hash;
use blackbox_collision::{BodyLimits, FitOptions, fit_body, write_bounds_pack};
use chameleon_vehicle::{Lod, Wheel};

use crate::source::SourceCar;

/// `simsurface` key of `carbody`, the surface every body node of a car's bounds has (`vlt_hash("carbody")`).
pub const CARBODY_SURFACE: u32 = 0x99DA_6531;

/// Where the points of the set came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// The collision mesh of the 2012 bundle.
    CollisionMesh,
    /// The visible body mesh: the bundle has no collision mesh.
    VisibleMesh,
}

/// A car's `BOUNDS.BIN`.
#[derive(Debug)]
pub struct Collision {
    pub bytes: Vec<u8>,
    pub basis: Basis,
    /// Half dimensions of the root box, metres (vehicle axes: x across, y up, z along).
    pub half: [f32; 3],
}

/// The bounds of `car` for the folder `folder`. `to_car` carries a point of the 2012 files (engine space: x left, y up,
/// z forward) into the car space the geometry is written in (x forward, y left, z up).
pub fn build(car: &SourceCar, to_car: &dyn Fn([f32; 3]) -> [f32; 3], folder: &str) -> Result<Collision> {
    let (mut points, basis) = match &car.collision {
        Some(soup) => (soup_points(soup), Basis::CollisionMesh),
        None => (visible_points(&car.vehicle.body), Basis::VisibleMesh),
    };
    for wheel in &car.vehicle.wheels {
        points.extend(tyre_corners(wheel));
    }
    let points: Vec<[f32; 3]> = points.into_iter().map(|p| to_physics(to_car(p))).collect();
    let options = FitOptions { surface: CARBODY_SURFACE, ..FitOptions::default() };
    let set = fit_body(key(folder), &points, &options).with_context(|| format!("fitting the bounds of {folder}"))?;
    set.check_vehicle_body(&BodyLimits::default()).with_context(|| format!("the bounds of {folder}"))?;
    let bytes =
        write_bounds_pack(std::slice::from_ref(&set)).with_context(|| format!("writing the bounds of {folder}"))?;
    Ok(Collision { bytes, basis, half: set.root().half_dimensions })
}

/// The name hash a car's set is looked up by: the folder in upper case.
pub fn key(folder: &str) -> u32 {
    vlt_hash(&folder.to_ascii_uppercase())
}

/// Car space (x forward, y left, z up) to the bounds space of vehicles (x right, y up, z forward).
fn to_physics(p: [f32; 3]) -> [f32; 3] {
    [-p[1], p[2], p[0]]
}

/// The vertices of the collision mesh and the centre of each of its triangles.
fn soup_points(soup: &chameleon_vehicle::PolySoup) -> Vec<[f32; 3]> {
    let mut points = soup.vertices.clone();
    for triangle in &soup.triangles {
        let [a, b, c] = triangle.map(|i| soup.vertices[i as usize]);
        points.push([(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0]);
    }
    points
}

/// Every vertex of the most detailed level of the body.
fn visible_points(body: &[Lod]) -> Vec<[f32; 3]> {
    body.first().into_iter().flatten().flat_map(|mesh| mesh.vertices.iter().map(|v| v.position)).collect()
}

/// The eight corners of the box around the most detailed level of a wheel, placed at its hub. The wheel is modelled as
/// a left wheel (outwards is +x); the right wheel is the mirror image.
fn tyre_corners(wheel: &Wheel) -> Vec<[f32; 3]> {
    let vertices: Vec<[f32; 3]> = visible_points(&wheel.lods);
    if vertices.is_empty() {
        return Vec::new();
    }
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in &vertices {
        for a in 0..3 {
            min[a] = min[a].min(v[a]);
            max[a] = max[a].max(v[a]);
        }
    }
    let mirror = if wheel.position[0] < 0.0 { -1.0 } else { 1.0 };
    let mut corners = Vec::with_capacity(8);
    for x in [min[0], max[0]] {
        // The lower face is at the hub: the box is the body's, and starts at its underside, not at the road.
        for y in [min[1].max(0.0), max[1]] {
            for z in [min[2], max[2]] {
                corners.push([wheel.position[0] + mirror * x, wheel.position[1] + y, wheel.position[2] + z]);
            }
        }
    }
    corners
}

#[cfg(test)]
mod tests;
