//! Oriented boxes and their overlap: the narrow phase of car against car.
//! Spec: `docs/specs/vehicle-rigid-body.md` (§6).

use glam::{Mat3, Vec3};

/// Edge-edge axes win over face axes only when clearly shallower: this keeps the contact stable.
const EDGE_BIAS: f32 = 1.05;
/// Cross products shorter than this are parallel edges and give no axis.
const MIN_AXIS: f32 = 1e-4;

/// A box: centre, orientation (columns are the world directions of its right, up and forward axes) and half
/// extents.
#[derive(Clone, Copy, Debug)]
pub struct Obb {
    pub centre: Vec3,
    pub axes: Mat3,
    pub half: Vec3,
}

/// Where two overlapping boxes touch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObbContact {
    /// Unit normal from the second box to the first.
    pub normal: Vec3,
    /// A point between the two boxes at the deepest part of the overlap.
    pub point: Vec3,
    /// How far the boxes overlap along the normal, metres.
    pub overlap: f32,
}

impl Obb {
    /// The corner of the box furthest along `direction`.
    pub fn support(&self, direction: Vec3) -> Vec3 {
        let sign = |axis: Vec3| if axis.dot(direction) >= 0.0 { 1.0 } else { -1.0 };
        let (x, y, z) = (self.axes.x_axis, self.axes.y_axis, self.axes.z_axis);
        self.centre + x * (self.half.x * sign(x)) + y * (self.half.y * sign(y)) + z * (self.half.z * sign(z))
    }

    /// Half the extent of the box along the unit vector `l`.
    fn radius(&self, l: Vec3) -> f32 {
        self.half.x * self.axes.x_axis.dot(l).abs()
            + self.half.y * self.axes.y_axis.dot(l).abs()
            + self.half.z * self.axes.z_axis.dot(l).abs()
    }
}

/// The overlap of `a` and `b` along the axis of least penetration (separating axis test), or `None` when
/// they are apart.
pub fn obb_contact(a: &Obb, b: &Obb) -> Option<ObbContact> {
    let d = a.centre - b.centre;
    let face = [a.axes.x_axis, a.axes.y_axis, a.axes.z_axis, b.axes.x_axis, b.axes.y_axis, b.axes.z_axis];
    let mut best: Option<(f32, Vec3)> = None;
    let mut test = |axis: Vec3, bias: f32| -> bool {
        let overlap = a.radius(axis) + b.radius(axis) - d.dot(axis).abs();
        if overlap < 0.0 {
            return false;
        }
        let score = overlap * bias;
        if best.is_none_or(|(s, _)| score < s) {
            let normal = if d.dot(axis) >= 0.0 { axis } else { -axis };
            best = Some((score, normal));
        }
        true
    };
    for axis in face {
        if !test(axis, 1.0) {
            return None;
        }
    }
    for fa in &face[..3] {
        for fb in &face[3..] {
            let cross = fa.cross(*fb);
            let len = cross.length();
            if len > MIN_AXIS && !test(cross / len, EDGE_BIAS) {
                return None;
            }
        }
    }
    let (_, normal) = best?;
    let overlap = a.radius(normal) + b.radius(normal) - d.dot(normal);
    let point = (a.support(-normal) + b.support(normal)) * 0.5;
    Some(ObbContact { normal, point, overlap })
}
