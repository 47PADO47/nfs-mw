//! Oriented boxes and displacement-aware face contacts.
//! Spec: `docs/specs/box-object-contacts.md`.

use glam::{Mat3, Vec3};

const SIDE_AXES: [[usize; 2]; 3] = [[1, 2], [0, 2], [0, 1]];

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
    /// Point on the incident object's surface.
    pub point: Vec3,
    /// How far the boxes overlap along the normal, metres.
    pub overlap: f32,
    /// Translate the contact point with the first body during penetration correction.
    pub point_on_first: bool,
}

impl Obb {
    /// The corner of the box furthest along `direction`.
    pub fn support(&self, direction: Vec3) -> Vec3 {
        let sign = |axis: Vec3| match axis.dot(direction) {
            v if v > 0.0 => 1.0,
            v if v < 0.0 => -1.0,
            _ => 0.0,
        };
        let (x, y, z) = (self.axes.x_axis, self.axes.y_axis, self.axes.z_axis);
        self.centre + x * (self.half.x * sign(x)) + y * (self.half.y * sign(y)) + z * (self.half.z * sign(z))
    }
}

/// Contact with no travel. Use [`box_contact`] for bodies moving within a fixed tick.
pub fn obb_contact(a: &Obb, b: &Obb) -> Option<ObbContact> {
    box_contact(a, b, Vec3::ZERO, Vec3::ZERO)
}

struct Face<'a> {
    reference: &'a Obb,
    incident: &'a Obb,
    axis: usize,
    normal: Vec3,
    travel: Vec3,
    contact: ObbContact,
}

/// Travel includes rotation of each primitive's offset, measured at the previous fixed-tick pose.
pub fn box_contact(a: &Obb, b: &Obb, travel_a: Vec3, travel_b: Vec3) -> Option<ObbContact> {
    if !travel_a.is_finite() || !travel_b.is_finite() || !valid(a) || !valid(b) {
        return None;
    }
    let mut best = None;
    let mut best_score = f32::NEG_INFINITY;
    for (reference, incident, dr, di, point_on_first) in
        [(a, b, travel_a, travel_b, false), (b, a, travel_b, travel_a, true)]
    {
        let mut delta = reference.centre.as_dvec3() - incident.centre.as_dvec3();
        delta.z = f64::from(delta.z as f32);
        let relative = dr.as_dvec3() - di.as_dvec3();
        let previous = (delta - relative).as_vec3().as_dvec3();
        for axis in [0, 2, 1] {
            let axis_vector = reference.axes.col(axis);
            let normal = match axis_vector.as_dvec3().dot(previous) < 0.0 {
                true => -axis_vector,
                false => axis_vector,
            };
            let n = normal.as_dvec3();
            let projections = (0..3).map(|i| n.dot((incident.axes.col(i) * incident.half[i]).as_dvec3()).abs());
            let signed = projections.fold((delta.dot(n) - f64::from(reference.half[axis])) as f32, |d, extent| {
                (f64::from(d) - extent) as f32
            });
            if signed > 0.0 {
                return None;
            }
            let score = f64::from(signed) - relative.dot(n);
            // The original compares an x87 intermediate with the stored single-precision score.
            if score <= f64::from(best_score) {
                continue;
            }
            best_score = score as f32;
            let output_normal = match point_on_first {
                true => -normal,
                false => normal,
            };
            best = Some(Face {
                reference,
                incident,
                axis,
                normal,
                travel: relative.as_vec3(),
                contact: ObbContact {
                    normal: output_normal,
                    point: incident.support(normal),
                    overlap: -signed,
                    point_on_first,
                },
            });
        }
    }
    best?.clip()
}

fn valid(b: &Obb) -> bool {
    b.centre.is_finite() && b.axes.is_finite() && b.half.is_finite() && b.half.min_element() >= 0.0
}

fn orient(axis: Vec3, delta: Vec3) -> Vec3 {
    if axis.as_dvec3().dot(delta.as_dvec3()) < 0.0 {
        return -axis;
    }
    axis
}

impl Face<'_> {
    fn clip(mut self) -> Option<ObbContact> {
        let delta = self.contact.point - self.reference.centre;
        let mut inside = 0;
        for side in SIDE_AXES[self.axis] {
            let normal = orient(self.reference.axes.col(side), delta + self.travel);
            if delta.as_dvec3().dot(normal.as_dvec3()) <= f64::from(self.reference.half[side]) {
                inside += 1;
                continue;
            }
            let plane =
                (self.reference.centre.as_dvec3() + normal.as_dvec3() * f64::from(self.reference.half[side])).as_vec3();
            for axis in 0..3 {
                let edge_axis = self.incident.axes.col(axis);
                let extent = (self.contact.point - self.incident.centre).as_dvec3().dot(edge_axis.as_dvec3());
                let reflected = (self.contact.point.as_dvec3() - edge_axis.as_dvec3() * (2.0 * extent)).as_vec3();
                let Some(point) = segment_plane(reflected, self.contact.point, plane, normal) else { continue };
                let offset = point - self.reference.centre;
                let depth = -offset.as_dvec3().dot(self.normal.as_dvec3());
                if depth >= f64::from(self.reference.half[self.axis]) {
                    continue;
                }
                let contains = SIDE_AXES[side].iter().all(|&i| {
                    if i == self.axis {
                        return depth <= f64::from(self.reference.half[i]);
                    }
                    offset.as_dvec3().dot(orient(self.reference.axes.col(i), offset + self.travel).as_dvec3())
                        <= f64::from(self.reference.half[i])
                });
                if !contains {
                    continue;
                }
                self.contact.point = point;
                self.contact.overlap = (f64::from(self.reference.half[self.axis]) - depth) as f32;
                return Some(self.contact);
            }
        }
        (inside == 2).then_some(self.contact)
    }
}

fn segment_plane(start: Vec3, end: Vec3, plane: Vec3, normal: Vec3) -> Option<Vec3> {
    let direction = end.as_dvec3() - start.as_dvec3();
    let denominator = direction.dot(normal.as_dvec3());
    if denominator.abs() <= 0.000001 {
        return None;
    }
    let t = ((plane.as_dvec3() - start.as_dvec3()).dot(normal.as_dvec3()) / denominator) as f32;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some((start.as_dvec3() + direction.as_vec3().as_dvec3() * f64::from(t)).as_vec3())
}

#[cfg(test)]
mod tests;
