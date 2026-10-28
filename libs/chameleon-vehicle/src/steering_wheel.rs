//! The steering wheel of a car: where it turns and how far. Spec: `docs/formats/chameleon-driver-animation.md`.
//!
//! The wheel is the part of the body skinned to the skeleton's `SteeringWheel` bone ([`crate::body_parts`]). The data names no
//! axis for it (the bone's rotation is not read, and the car's `SteeringWheelBehaviour` object holds only a name) and no lock
//! angle there, so the axis is read from the wheel's own shape: a steering wheel is a flat disc, and the direction in which its
//! vertices vary least is the column. The lock angle is the one number the handling object keeps for the wheel
//! (`SteeringWheelPhysics`, +0x70: 400.0 on most cars, 300.0 on a few) **[unconfirmed meaning]**.

use chameleon_bundle::Bundle;

use crate::genesys::{TypeNames, find_object};

/// The name of the skeleton bone the wheel is skinned to.
pub const BONE: &str = "SteeringWheel";
/// The degrees the wheel turns at full steering, each way, for a car whose data gives no number.
pub const DEFAULT_LOCK_DEGREES: f32 = 450.0;
/// The type of the handling object's wheel structure, and the offset in it of the lock angle.
const PHYSICS: &str = "Genesys.Gen.SteeringWheelPhysics";
const LOCK_AT: usize = 0x70;
/// Least vertices a wheel's shape is read from.
const MIN_POINTS: usize = 12;
/// The shape is a disc when the least variance is under this share of the middle one.
const FLAT: f64 = 0.5;

/// Where a steering wheel turns, in the engine's space (x left, y up, z forward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelFrame {
    /// The centre of the rim, on the column.
    pub pivot: [f32; 3],
    /// The column's direction, a unit vector pointing forward (away from the driver), so a right-handed turn about it is
    /// clockwise as the driver sees it.
    pub axis: [f32; 3],
}

/// The degrees the wheel of the car in `bundle` turns at full steering, each way: the handling object's number when it has a
/// sensible one (90 to 1080), else `None`.
pub fn lock_degrees(bundle: &Bundle) -> Option<f32> {
    let types = TypeNames::read(bundle).ok()?;
    let object = find_object(bundle, &types, PHYSICS).ok()?;
    let at = object.at(PHYSICS, 0)?;
    let lock = object.f32(at + LOCK_AT).ok()?;
    (lock.is_finite() && (90.0..=1080.0).contains(&lock)).then_some(lock)
}

/// The pivot and axis of a steering wheel from its vertices. `None` for too few points, or a shape that is not flat.
pub fn wheel_frame(points: &[[f32; 3]]) -> Option<WheelFrame> {
    if points.len() < MIN_POINTS {
        return None;
    }
    let count = points.len() as f64;
    let centre: [f64; 3] = std::array::from_fn(|k| points.iter().map(|p| f64::from(p[k])).sum::<f64>() / count);
    let mut covariance = [[0.0f64; 3]; 3];
    for p in points {
        let d: [f64; 3] = std::array::from_fn(|k| f64::from(p[k]) - centre[k]);
        for (i, row) in covariance.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell += d[i] * d[j] / count;
            }
        }
    }
    let (values, vectors) = eigen(covariance);
    // Largest variance first: the two in-plane directions, then the column.
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let [u, v, n] = order;
    if values[v] <= 0.0 || values[n] > FLAT * values[v] {
        return None;
    }
    let (u, v, mut axis) = (column(&vectors, u), column(&vectors, v), column(&vectors, n));
    if axis[2] < 0.0 {
        axis = axis.map(|c| -c);
    }
    // The middle of the rim in the wheel's plane (its box, so a flat bottom or a spoke does not pull it), on the column line
    // through the centre of the vertices.
    let along = |direction: [f64; 3]| -> f64 {
        let (low, high) = points.iter().fold((f64::MAX, f64::MIN), |(low, high), p| {
            let s: f64 = (0..3).map(|k| (f64::from(p[k]) - centre[k]) * direction[k]).sum();
            (low.min(s), high.max(s))
        });
        (low + high) * 0.5
    };
    let (mid_u, mid_v) = (along(u), along(v));
    let pivot = std::array::from_fn(|k| (centre[k] + u[k] * mid_u + v[k] * mid_v) as f32);
    Some(WheelFrame { pivot, axis: axis.map(|c| c as f32) })
}

fn column(vectors: &[[f64; 3]; 3], k: usize) -> [f64; 3] {
    [vectors[0][k], vectors[1][k], vectors[2][k]]
}

/// Eigenvalues and eigenvectors (as the columns of the matrix) of a symmetric 3x3 matrix, by Jacobi rotations.
fn eigen(mut a: [[f64; 3]; 3]) -> ([f64; 3], [[f64; 3]; 3]) {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..32 {
        let off = a[0][1].abs() + a[0][2].abs() + a[1][2].abs();
        if off < 1e-14 {
            break;
        }
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            if a[p][q].abs() < 1e-300 {
                continue;
            }
            let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = match theta == 0.0 {
                true => 1.0,
                false => theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt()),
            };
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            for row in &mut a {
                let (rp, rq) = (row[p], row[q]);
                row[p] = c * rp - s * rq;
                row[q] = s * rp + c * rq;
            }
            for k in 0..3 {
                let (pk, qk) = (a[p][k], a[q][k]);
                a[p][k] = c * pk - s * qk;
                a[q][k] = s * pk + c * qk;
            }
            for row in &mut v {
                let (vp, vq) = (row[p], row[q]);
                row[p] = c * vp - s * vq;
                row[q] = s * vp + c * vq;
            }
        }
    }
    ([a[0][0], a[1][1], a[2][2]], v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    }

    fn unit(a: [f32; 3]) -> [f32; 3] {
        let length = a.iter().map(|c| c * c).sum::<f32>().sqrt();
        a.map(|c| c / length)
    }

    /// A rim of radius `r` in the plane normal to `normal`, centred on `centre`, with a hub and a little thickness.
    fn wheel(centre: [f32; 3], normal: [f32; 3], r: f32) -> Vec<[f32; 3]> {
        let n = unit(normal);
        let helper = if n[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
        let u = unit(cross(n, helper));
        let v = cross(n, u);
        let mut points = Vec::new();
        for step in 0..64 {
            let angle = step as f32 / 64.0 * std::f32::consts::TAU;
            for (radius, depth) in [(r, 0.0), (r, 0.02), (r * 0.3, 0.03)] {
                points.push(std::array::from_fn(|k| {
                    centre[k] + radius * (angle.cos() * u[k] + angle.sin() * v[k]) + depth * n[k]
                }));
            }
        }
        points
    }

    #[test]
    fn the_column_is_the_direction_a_flat_wheel_varies_least_in_and_it_points_forward() {
        let tilted = unit([0.0, 0.5, 1.0]);
        let frame = wheel_frame(&wheel([0.36, 0.9, 0.2], tilted, 0.18)).expect("a disc");
        for k in 0..3 {
            assert!((frame.axis[k] - tilted[k]).abs() < 0.02, "{:?}", frame.axis);
        }
        // The same wheel described with the opposite normal still points forward.
        let flipped = wheel_frame(&wheel([0.36, 0.9, 0.2], [0.0, -0.5, -1.0], 0.18)).unwrap();
        assert!(flipped.axis[2] > 0.0);
    }

    #[test]
    fn the_pivot_is_the_middle_of_the_rim() {
        let frame = wheel_frame(&wheel([0.36, 0.9, 0.2], [0.0, 0.4, 1.0], 0.18)).unwrap();
        let near = frame.pivot.iter().zip([0.36, 0.9, 0.2]).map(|(p, c)| (p - c).powi(2)).sum::<f32>().sqrt();
        assert!(near < 0.03, "{:?}", frame.pivot);
    }

    #[test]
    fn a_blob_or_a_handful_of_points_is_not_a_wheel() {
        assert!(wheel_frame(&[[0.0; 3]; 4]).is_none());
        let cube: Vec<[f32; 3]> = (0..64).map(|i| [(i % 4) as f32, ((i / 4) % 4) as f32, (i / 16) as f32]).collect();
        assert!(wheel_frame(&cube).is_none(), "no flat direction");
    }

    /// With the install: the handling object of the cars gives a lock angle.
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn the_lock_angle_of_every_car_is_in_the_handling_object() {
        let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
        let dir = std::path::Path::new(&root).join("VEHICLES");
        let (mut seen, mut locks) = (0, std::collections::BTreeSet::new());
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let file = entry.file_name().to_string_lossy().to_uppercase();
            if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
                continue;
            }
            let data = std::fs::read(entry.path()).unwrap();
            let bundle = Bundle::parse(&data).unwrap();
            let Some(lock) = lock_degrees(&bundle) else { continue };
            seen += 1;
            locks.insert(lock as i32);
        }
        assert!(seen >= 90, "{seen} cars have a lock angle");
        // 400 on most cars and 300 on a few: a per-car number, so it is a lock angle and not a constant of the engine.
        assert!(locks.iter().all(|lock| (200..=500).contains(lock)), "{locks:?}");
    }
}
