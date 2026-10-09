//! The curvature number the driver limits its speed with.
//! Spec: `docs/specs/ai-road-nav-trail.md` (§6). The original marks the function unsolved: the apex term
//! is approximate.

use glam::{Vec2, Vec3};

use super::{Cookie, Occlusion, cross, xz};

/// Each cookie's curvature is clamped to this (1/m) before averaging.
const CURVATURE_CLAMP: f32 = 0.01;

/// The larger of the road term (length-weighted mean of the cookies' curvature from `from` on) and the
/// apex term (an occluded view of the corridor), in 1/m.
pub fn trail_curvature(
    cookies: &[Cookie],
    from: usize,
    car: Vec3,
    occlusion: &Occlusion,
    cursor: Vec3,
    own_speed_along: f32,
) -> f32 {
    road_term(cookies, from).max(apex_term(cookies.len(), car, occlusion, cursor, own_speed_along))
}

fn road_term(cookies: &[Cookie], from: usize) -> f32 {
    let usable = cookies.get(from..).unwrap_or(&[]);
    if usable.len() < 2 {
        return 0.0;
    }
    let (mut sum, mut length) = (0.0, 0.0);
    for c in &usable[1..] {
        sum += c.curvature.abs().min(CURVATURE_CLAMP) * c.length;
        length += c.length;
    }
    match length > 0.0 {
        true => sum / length,
        false => 0.0,
    }
}

fn apex_term(count: usize, car: Vec3, occlusion: &Occlusion, cursor: Vec3, own_speed_along: f32) -> f32 {
    if count <= 2 || !occlusion.occluded() || occlusion.from_behind {
        return 0.0;
    }
    let car2 = xz(car);
    let to_target = xz(occlusion.position) - car2;
    let distance = to_target.length();
    if distance <= 1.0 {
        return 0.0;
    }
    let width = occlusion.apex_width.max(distance).max(1.0);
    let apex = xz(occlusion.apex);
    let (a, b) = (to_target / distance, (xz(cursor) - apex).try_normalize().unwrap_or(Vec2::Y));
    let mut sina = cross(a, b).clamp(-1.0, 1.0).asin();
    if a.dot(b) < 0.0 {
        sina = std::f32::consts::PI - sina;
    }
    let mut value = (sina * sina.min(std::f32::consts::FRAC_PI_2).sin()).clamp(0.0, std::f32::consts::PI) / width;
    if occlusion.avoidable != 0 {
        let ratio = (occlusion.closing_speed / own_speed_along.abs().max(1e-3)).clamp(0.0, 1.0);
        value *= ratio * ratio;
    }
    value
}
