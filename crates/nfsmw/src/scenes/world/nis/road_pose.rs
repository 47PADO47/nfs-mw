//! Animation supplies heading and planar position; the wheels follow the road
//! (docs/specs/nis-events-3.md section 3).
use super::car_world::heading_of;
use super::cars::{CarSpot, Target};
use crate::scenes::world::WorldScene;
use glam::{Mat3, Quat, Vec3};

impl WorldScene {
    pub(super) fn target_treads(&self, target: Target) -> Option<([Vec3; 4], f32, bool)> {
        match target {
            Target::Player => {
                let car = self.players.lead()?;
                car.scene_treads(car.pose())
            }
            Target::Traffic(i) | Target::Spawned(i) => {
                let traffic = self.traffic.as_ref()?;
                traffic.scene_treads(i, traffic.car_pose(i)?)
            }
        }
    }
}

/// Limit recovery to a nearby street above a buried clip. Require a tyre
/// surface at the actor's own XY, near that deck's elevation.
pub(super) fn mapped_ground(at: Vec3, street: Vec3, probe: impl Fn(f32, f32) -> Option<f32>) -> Option<f32> {
    let rise = street.z - at.z;
    if !(0.0..=15.0).contains(&rise) || at.truncate().distance(street.truncate()) > 16.0 {
        return None;
    }
    probe(street.z + 2.0, street.z - 2.0)
}

/// Sample the tread footprint on the centre's road deck, fit its grade and
/// bank, then sample at the rotated tyre positions for individual wheel travel.
pub(super) fn place(
    spot: CarSpot,
    treads: [Vec3; 4],
    scale: f32,
    road: f32,
    ground: impl Fn(Vec3, f32) -> Option<f32>,
) -> Option<(CarSpot, [f32; 4])> {
    let yaw = Quat::from_rotation_z(heading_of(spot.rotation));
    let max_slope = 30.0_f32.to_radians().tan();
    let sample =
        |rotation: Quat| treads.map(|p| ground(spot.position + rotation * p, p.truncate().length() * max_slope + 0.5));
    let heights = sample(yaw);
    let points = std::array::from_fn::<_, 4, _>(|i| heights[i].map(|z| treads[i].with_z(z - road - treads[i].z)));
    let mut normal = Vec3::ZERO;
    // Each triangle contributes its area-weighted upward normal, including
    // the remaining triangle when one wheel projects beyond the road edge.
    for [a, b, c] in [[0, 1, 2], [0, 2, 3], [0, 1, 3], [1, 2, 3]] {
        if let (Some(a), Some(b), Some(c)) = (points[a], points[b], points[c]) {
            let n = (b - a).cross(c - a);
            normal += n * n.z.signum();
        }
    }
    let mut rotation = yaw;
    if normal.z > 1e-5 {
        let slope = (-normal.truncate() / normal.z).clamp_length_max(max_slope);
        let up = Vec3::new(-slope.x, -slope.y, 1.0).normalize();
        let forward = Vec3::new(1.0, 0.0, slope.x).normalize();
        rotation = yaw * Quat::from_mat3(&Mat3::from_cols(forward, up.cross(forward), up));
    }
    let support = sample(rotation);
    let offsets = std::array::from_fn::<_, 4, _>(|i| support[i].map(|h| h - (rotation * treads[i]).z));
    let count = offsets.iter().flatten().count();
    if count == 0 || scale <= 0.0 {
        return None;
    }
    let z = offsets.iter().flatten().sum::<f32>() / count as f32;
    let vertical = (rotation * Vec3::Z).z * scale;
    let travel = offsets.map(|h| h.map_or(0.0, |h| ((h - z) / vertical).clamp(-0.25, 0.25)));
    Some((CarSpot { position: spot.position.with_z(z), rotation }, travel))
}

/// Keep an upright released actor's visible tyres outside the road. The solver
/// retains its position, velocity and collision rotation. A wheel clear of
/// the surface keeps its simulated droop, including over a jump.
pub(super) fn released(
    spot: &mut CarSpot,
    travel: &mut [f32; 4],
    treads: [Vec3; 4],
    scale: f32,
    ground: impl Fn(Vec3) -> Option<f32>,
) {
    let up = spot.rotation * Vec3::Z;
    if up.z < 0.5 || scale <= 0.0 {
        return;
    }
    let rest = treads.map(|p| spot.position + spot.rotation * p);
    let heights = std::array::from_fn::<_, 4, _>(|i| ground(rest[i] + up * travel[i] * scale));
    // A common penetration at rest comes from the body's model origin.
    // Absorb only the part shared by all four wheels in the body position.
    let lift = (0..4).map(|i| heights[i].map_or(0.0, |h| h - rest[i].z)).fold(f32::INFINITY, f32::min).clamp(0.0, 0.5);
    spot.position.z += lift;
    for i in 0..4 {
        if let Some(h) = heights[i] {
            let minimum = (h - rest[i].z - lift) / (scale * up.z);
            travel[i] = travel[i].max(minimum.min(0.25));
        }
    }
}

#[cfg(test)]
#[path = "road_pose_tests.rs"]
mod tests;
