//! Static scenery overlap helpers in render coordinates.

use glam::{Mat4, Vec3};
use nfsmw_data::world::PropKind;

/// An oriented box: centre, three orthonormal axes and the half extent along each.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    pub centre: Vec3,
    pub axes: [Vec3; 3],
    pub half: Vec3,
}

impl Obb {
    /// The box with local half extents `half` placed by `to_world` (its axes may be scaled).
    pub(super) fn from_matrix(to_world: &Mat4, half: Vec3) -> Self {
        let col = |m: glam::Vec4| m.truncate();
        let axes = [col(to_world.x_axis), col(to_world.y_axis), col(to_world.z_axis)];
        let lengths = Vec3::new(axes[0].length(), axes[1].length(), axes[2].length());
        Self { centre: col(to_world.w_axis), axes: axes.map(|a| a.normalize_or_zero()), half: half * lengths }
    }

    /// How far the box reaches along the unit vector `l` from its centre.
    pub fn support(&self, l: Vec3) -> f32 {
        (0..3).map(|i| self.half[i] * self.axes[i].dot(l).abs()).sum()
    }
}

/// A prop the car's box overlaps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropOverlap {
    pub id: u32,
    pub kind: PropKind,
    /// Unit axis of least overlap, from the prop towards the car.
    pub normal: Vec3,
    pub depth: f32,
}

/// Whether two oriented boxes overlap (separating axis test): the axis of least overlap, pointing
/// from `b` to `a`, and the overlap along it.
pub(super) fn obb_overlap(a: &Obb, b: &Obb) -> Option<(Vec3, f32)> {
    let mut best: Option<(Vec3, f32)> = None;
    let offset = a.centre - b.centre;
    let mut test = |axis: Vec3| -> bool {
        let len = axis.length();
        if len < 1e-5 {
            return true; // parallel edges: no axis
        }
        let l = axis / len;
        let overlap = a.support(l) + b.support(l) - offset.dot(l).abs();
        if overlap < 0.0 {
            return false;
        }
        if best.is_none_or(|(_, d)| overlap < d) {
            best = Some((if offset.dot(l) < 0.0 { -l } else { l }, overlap));
        }
        true
    };
    for i in 0..3 {
        if !test(a.axes[i]) || !test(b.axes[i]) {
            return None;
        }
    }
    for i in 0..3 {
        for j in 0..3 {
            if !test(a.axes[i].cross(b.axes[j])) {
                return None;
            }
        }
    }
    best
}
