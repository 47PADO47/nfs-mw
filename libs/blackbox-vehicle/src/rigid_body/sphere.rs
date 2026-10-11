//! Sphere contact and relative sphere/box sweeps; `docs/specs/sphere-contacts.md`.

use glam::Vec3;

use super::{Obb, ObbContact};

const EPSILON: f32 = 0.000001;

#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub centre: Vec3,
    pub radius: f32,
}

pub fn sphere_contact(a: Sphere, b: Sphere) -> Option<ObbContact> {
    if !a.centre.is_finite()
        || !b.centre.is_finite()
        || !a.radius.is_finite()
        || !b.radius.is_finite()
        || a.radius <= 0.0
        || b.radius <= 0.0
    {
        return None;
    }
    let distance = a.centre.distance(b.centre);
    if distance == 0.0 {
        // The original leaves point components unwritten here. Keep a deterministic finite point.
        return Some(ObbContact { normal: Vec3::NEG_Y, point: a.centre, overlap: a.radius, point_on_first: true });
    }
    let overlap = a.radius + b.radius - distance;
    if overlap <= 0.0 || !overlap.is_finite() {
        return None;
    }
    let normal = (a.centre - b.centre) / distance;
    Some(ObbContact { normal, point: a.centre - normal * a.radius, overlap, point_on_first: true })
}

/// `travel` is sphere displacement minus box displacement, including each local offset's rotation.
pub fn sphere_box_contact(sphere: Sphere, b: &Obb, travel: Vec3) -> Option<ObbContact> {
    let radius = sphere.radius;
    if radius <= 0.0
        || !radius.is_finite()
        || !sphere.centre.is_finite()
        || !travel.is_finite()
        || !b.centre.is_finite()
        || !b.half.is_finite()
        || b.half.min_element() < 0.0
    {
        return None;
    }
    let local = b.axes.transpose() * (sphere.centre - b.centre);
    let previous = local - b.axes.transpose() * travel;
    let sign = Vec3::from_array(previous.to_array().map(|v| match v > 0.0 {
        true => 1.0,
        false => -1.0,
    }));
    let current = local * sign;
    let before = previous * sign;
    if current.cmpge(b.half + radius).any() {
        return None;
    }
    let depth = b.half + radius - current;
    let old_depth = b.half + radius - before;
    let area = Vec3::new(b.half.y * b.half.z, b.half.x * b.half.z, b.half.x * b.half.y);
    let outside = before.cmpgt(b.half);
    if current.cmple(b.half).all() || !outside.any() {
        let axis = (0..3).filter(|&i| area[i] > 0.0).min_by(|&i, &j| old_depth[i].total_cmp(&old_depth[j]))?;
        let normal = b.axes.col(axis) * sign[axis];
        return Some(ObbContact {
            normal,
            point: sphere.centre + normal * depth[axis],
            overlap: depth[axis],
            point_on_first: false,
        });
    }
    let offset = b.axes * Vec3::select(outside, (b.half - current) * sign, Vec3::ZERO);
    let point = sphere.centre + offset;
    let length2 = travel.length_squared();
    let projection = (offset + travel).dot(travel);
    let full_volume = b.half.min_element() > 0.0;
    let flat_axis = || (0..3).find(|&i| outside.test(i) && area[i] > 0.0);
    if length2 <= EPSILON || projection <= 0.0 || projection >= length2 {
        if full_volume {
            let distance = offset.length();
            if distance <= EPSILON {
                return None;
            }
            let direction = offset / distance;
            let overlap = radius - distance;
            if radius + direction.dot(travel) < distance || overlap <= 0.0 {
                return None;
            }
            return Some(ObbContact { normal: -direction, point, overlap, point_on_first: false });
        }
        let offset2 = offset.length_squared();
        if offset2 >= radius * radius {
            return None;
        }
        let axis = flat_axis()?;
        let normal = b.axes.col(axis) * sign[axis];
        let along = offset.dot(normal);
        let overlap = along + (along * along + radius * radius - offset2).sqrt();
        if overlap <= EPSILON {
            return None;
        }
        return Some(ObbContact { normal, point, overlap, point_on_first: false });
    }
    let closest = sphere.centre - travel + travel * (projection / length2);
    let distance2 = closest.distance_squared(point);
    let radicand = radius * radius - distance2;
    if radicand < EPSILON {
        return None;
    }
    if !full_volume {
        let axis = flat_axis()?;
        return Some(ObbContact {
            normal: b.axes.col(axis) * sign[axis],
            point,
            overlap: depth[axis],
            point_on_first: false,
        });
    }
    let entry = closest - travel * (radicand / length2).sqrt();
    let normal = (entry - point).try_normalize()?;
    let overlap = radius - distance2.sqrt();
    if overlap <= 0.0 {
        return None;
    }
    Some(ObbContact { normal, point, overlap, point_on_first: false })
}

#[cfg(test)]
mod tests;
