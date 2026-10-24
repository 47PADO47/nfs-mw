//! Horizontal capsule separation for chase and collapse. Spec: ai-pursuit-tactics §3.5.
//! Capsule axes use horizontal body headings; full pitch and roll are unavailable in the avoidable interface.

use blackbox_roads::Body;
use glam::Vec2;

fn xz(v: glam::Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

fn capsule(body: &Body) -> (Vec2, Vec2, f32) {
    let a = body.half_width.min(body.half_length).max(0.01);
    let b = body.half_width.max(body.half_length).max(a);
    let axis = match body.half_length >= body.half_width {
        true => body.forward,
        false => Vec2::new(body.forward.y, -body.forward.x),
    };
    let c = a * a / b;
    let extent = axis.normalize_or_zero() * (b - c);
    (xz(body.position) - extent, xz(body.position) + extent, (c * c + a * a).sqrt())
}

fn closest(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let axis = b - a;
    a + axis * ((p - a).dot(axis) / axis.length_squared().max(0.0001)).clamp(0.0, 1.0)
}

fn distance(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Vec2 {
    let u = b - a;
    let v = d - c;
    let cross = u.perp_dot(v);
    if cross.abs() > 0.0001 {
        let t = (c - a).perp_dot(v) / cross;
        let s = (c - a).perp_dot(u) / cross;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&s) {
            return Vec2::ZERO;
        }
    }
    [a - closest(a, c, d), b - closest(b, c, d), closest(c, a, b) - c, closest(d, a, b) - d]
        .into_iter()
        .min_by(|x, y| x.length_squared().total_cmp(&y.length_squared()))
        .unwrap_or_default()
}

/// Push away from other vehicle capsules, with prediction from relative velocity and a per-vehicle 160 km/h cap.
/// Exact overlapping axes have no geometric normal and contribute zero; collision resolution handles overlap.
pub(in crate::scenes::world::ai) fn steer(
    me: &Body,
    others: impl IntoIterator<Item = Body>,
    absolute: f32,
    relative: f32,
) -> Vec2 {
    let (a, b, radius) = capsule(me);
    let mut result = Vec2::ZERO;
    for other in others {
        let (c, d, r) = capsule(&other);
        let planar = distance(a, b, c, d);
        let separation = glam::Vec3::new(planar.x, me.position.y - other.position.y, planar.y);
        let delta_v = me.velocity - other.velocity;
        let centre = me.position - other.position;
        let time = -centre.dot(delta_v) / delta_v.length_squared().max(0.0001);
        let mut scale = absolute;
        if time > 0.05 {
            let future_sq = (centre + delta_v * time).length_squared();
            scale += relative / (0.5 * future_sq + time).powi(2).max(0.0001);
        }
        let push = separation * (radius + r).powi(2) / separation.length().powi(3).max(0.1);
        result += xz(push * scale.min((160.0 / 3.6) / push.length().max(0.00001)));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn body(x: f32, z: f32) -> Body {
        Body {
            position: glam::Vec3::new(x, 0.0, z),
            velocity: glam::Vec3::ZERO,
            forward: Vec2::Y,
            half_width: 1.0,
            half_length: 2.0,
        }
    }
    #[test]
    fn adjacent_cars_push_apart_without_longitudinal_bias() {
        let push = steer(&body(0.0, 0.0), [body(2.5, 0.0)], 2.7, 5.3);
        assert!(push.x < 0.0 && push.y.abs() < 0.001, "{push:?}");
    }
    #[test]
    fn converging_vehicles_have_more_avoidance_than_receding_ones() {
        let mut me = body(0.0, 0.0);
        me.velocity.x = 2.0;
        let other = body(5.0, 0.0);
        let approaching = steer(&me, [other], 2.7, 5.3).length();
        me.velocity.x = -2.0;
        assert!(approaching > steer(&me, [other], 2.7, 5.3).length());
    }
    #[test]
    fn vertical_distance_attenuates_another_decks_horizontal_separation() {
        let mut other = body(1.0, 0.0);
        let same_deck = steer(&body(0.0, 0.0), [other], 2.7, 5.3);
        other.position.y = 8.0;
        let other_deck = steer(&body(0.0, 0.0), [other], 2.7, 5.3);
        assert!(other_deck.x < 0.0, "the source has no arbitrary vertical exclusion gate");
        assert!(other_deck.length() < same_deck.length() * 0.01);
    }
}
