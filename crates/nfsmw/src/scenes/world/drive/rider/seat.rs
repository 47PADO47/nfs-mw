//! Sitting the rider on the seat of his bike.
//!
//! The rider's root bone is put at the bike's rider socket, which is not on the seat (it is 3 to 15 cm above its top, a different
//! height on every bike). The converter measures the seat's top along the bike (`contacts.json`, `seat_surface`); the vertices of
//! the rider's skin that carry his buttocks (those skinned to the bones `attach.json` names under `seat`) are taken once, in the
//! frame of the first of those bones, and each frame the body is moved along the bike's up axis until the lowest of them is
//! [`SeatRig::sink`](super::attach::SeatRig) below the surface: he sits on the seat, into its padding, whatever the bike's height.
//! Points ahead of or behind the measured profile are left out, so a knee against the tank does not lift him.

use blackbox_skeleton::{MAX_INFLUENCES, Pack, Skeleton, Transform};
use glam::{Mat4, Vec3};

use super::attach::Attach;

/// The share of a vertex's weights on the seat bones from which it counts as carrying the buttocks.
pub const SEAT_WEIGHT: f32 = 0.5;
/// The most the body is moved to sit, metres: a bound for damaged data, not a modelling value.
pub const MAX_SHIFT_M: f32 = 0.2;

/// The rider's contact with the seat.
#[derive(Debug, Clone, PartialEq)]
pub struct Seat {
    /// The bone the points are fixed to.
    anchor: usize,
    /// The points that rest on the seat, in the anchor bone's frame.
    points: Vec<Vec3>,
    /// The top of the seat, `[x, z]` in the bike skeleton's frame, x ascending.
    surface: Vec<[f32; 2]>,
    /// How far the lowest point sinks below the surface.
    sink: f32,
}

/// The height of the profile `surface` (x ascending) at `x`, by linear interpolation; `None` outside it.
pub fn height(surface: &[[f32; 2]], x: f32) -> Option<f32> {
    let (first, last) = (surface.first()?, surface.last()?);
    if surface.len() < 2 || !(first[0]..=last[0]).contains(&x) {
        return None;
    }
    let after = surface.partition_point(|p| p[0] < x).clamp(1, surface.len() - 1);
    let (a, b) = (surface[after - 1], surface[after]);
    let along = ((x - a[0]) / (b[0] - a[0])).clamp(0.0, 1.0);
    Some(a[1] + (b[1] - a[1]) * along)
}

impl Seat {
    /// The seat of the rider `pack` holds on the bike `attach` describes. `None` when the file names no seat bones, the bike has
    /// no measured surface or no vertex rests on it.
    pub fn new(attach: &Attach, pack: &Pack) -> Option<Self> {
        let rig = attach.seat.as_ref()?;
        if attach.seat_surface.len() < 2 {
            return None;
        }
        let skeleton: &Skeleton = &pack.skeleton;
        let bones: Vec<usize> = rig.bones.iter().filter_map(|name| skeleton.find(name)).collect();
        let anchor = *bones.first()?;
        let mut points = Vec::new();
        for mesh in &pack.meshes {
            let Some(own) = mesh.bone_map.iter().position(|b| *b == Some(anchor)) else { continue };
            let Some(lod) = mesh.lods.first() else { continue };
            let bind = mesh.inverse_bind[own];
            for vertex in &lod.vertices {
                let total = vertex.influences.total();
                let on_seat: f32 = (0..MAX_INFLUENCES)
                    .filter(|&slot| {
                        let bone = mesh.bone_map.get(usize::from(vertex.influences.bones[slot])).copied().flatten();
                        bone.is_some_and(|b| bones.contains(&b))
                    })
                    .map(|slot| vertex.influences.weights[slot])
                    .sum();
                if total > 0.0 && on_seat >= SEAT_WEIGHT * total {
                    points.push(bind.transform_point3(vertex.position));
                }
            }
        }
        if points.is_empty() {
            return None;
        }
        Some(Self { anchor, points, surface: attach.seat_surface.clone(), sink: rig.sink })
    }

    /// A seat made of the given points (for tests).
    #[cfg(test)]
    pub fn from_points(anchor: usize, points: Vec<Vec3>, surface: Vec<[f32; 2]>, sink: f32) -> Self {
        Self { anchor, points, surface, sink }
    }

    /// The height of the lowest seat point above the surface under it (negative: below it), for the pose `component`, the rider's
    /// component space taken into the bike skeleton's frame by `to_bike`. `None` when no point is over the measured profile.
    pub fn clearance(&self, component: &[Transform], to_bike: &Mat4) -> Option<f32> {
        let to_bike = *to_bike * component.get(self.anchor)?.to_mat4();
        self.points
            .iter()
            .filter_map(|&local| {
                let at = to_bike.transform_point3(local);
                Some(at.z - height(&self.surface, at.x)?)
            })
            .min_by(f32::total_cmp)
    }

    /// How far to lower the body (negative: raise it), along the bike's up axis, for the lowest seat point to sink `sink` into
    /// the seat; held to [`MAX_SHIFT_M`]. `None` when nothing is over the profile.
    pub fn drop(&self, component: &[Transform], to_bike: &Mat4) -> Option<f32> {
        let clearance = self.clearance(component, to_bike)?;
        Some((clearance + self.sink).clamp(-MAX_SHIFT_M, MAX_SHIFT_M))
    }
}

#[cfg(test)]
#[path = "seat_tests.rs"]
mod tests;
