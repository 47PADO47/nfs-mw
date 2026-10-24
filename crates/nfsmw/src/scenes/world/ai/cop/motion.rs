//! Path-chase measurements, forward cursor advancement and braking before turning.
//! Specs: ai-pursuit-driving-integration.md, ai-pursuit-reinforcement-driving.md.

use blackbox_roads::Body;
use glam::{Vec2, Vec3};

use super::{CopThink, close};

/// Forward tangent travel to the horizontal look-ahead circle's far intersection.
pub(super) fn nav_advance(offset: Vec3, forward: Vec3, look: f32) -> f32 {
    let forward = close::xz(forward.normalize_or_zero());
    let offset = close::xz(offset);
    let a = forward.length_squared();
    let denominator = 2.0 * a;
    if denominator <= 0.00001 {
        return 0.0;
    }
    let b = 2.0 * offset.dot(forward);
    let c = offset.length_squared() - look * look;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return 0.0;
    }
    let distance = (discriminant.sqrt() - b) / denominator;
    if !distance.is_finite() {
        return 0.0;
    }
    distance.max(0.0)
}

pub(super) fn speed_vectors(me: &Body, target: &Body, last_find: Vec3) -> (Vec2, Vec2) {
    let offset = close::xz(me.position - target.position);
    let seek = close::xz(last_find - target.position).try_normalize().unwrap_or(target.forward);
    (offset, seek)
}

pub(super) fn needs_turnaround(me: &Body, destination: Vec3) -> bool {
    let speed = me.velocity.length();
    let offset = destination - me.position;
    speed > 40.0 / 3.6 && me.velocity.dot(offset) < -0.3 * speed * offset.length()
}

pub(super) fn brake_on_future_road(future: Vec3) -> CopThink {
    CopThink { target: future, speed: 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(position: Vec3, velocity: Vec3) -> Body {
        Body { position, velocity, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
    }

    #[test]
    fn cursor_advance_uses_the_forward_circle_intersection_even_after_the_car_passes_it() {
        assert!((nav_advance(-Vec3::Z * 40.0, Vec3::Z, 40.0) - 80.0).abs() < 0.0001);
        assert!((nav_advance(Vec3::new(30.0, 0.0, -40.0), Vec3::Z * 100.0, 50.0) - 80.0).abs() < 0.0001);
        assert_eq!(nav_advance(Vec3::Z * 60.0, Vec3::Z, 50.0), 0.0);
        assert_eq!(nav_advance(Vec3::X * 60.0, Vec3::Z, 50.0), 0.0);
        assert_eq!(nav_advance(Vec3::ZERO, Vec3::ZERO, 50.0), 0.0);
    }

    #[test]
    fn cursor_advance_normalizes_the_grade_before_forming_the_planar_quadratic() {
        let forward = Vec3::new(0.0, 6.0, 8.0);
        let offset = Vec3::new(0.0, 100.0, -40.0);
        assert!((nav_advance(offset, forward, 40.0) - 100.0).abs() < 0.0001);
        assert!((nav_advance(offset, Vec3::Z, 40.0) - 80.0).abs() < 0.0001);
    }

    #[test]
    fn catchup_uses_the_raw_target_and_the_path_horizon() {
        let me = body(Vec3::ZERO, Vec3::Z * 20.0);
        let target = body(Vec3::Z * 40.0, Vec3::Z * 20.0);
        let horizon = target.position + Vec3::X * 80.0;
        let (offset, seek) = speed_vectors(&me, &target, horizon);
        assert_eq!(offset, Vec2::new(0.0, -40.0));
        assert_eq!(seek, Vec2::X);
        assert_ne!(seek, target.forward);
    }

    #[test]
    fn a_reverse_path_at_speed_brakes_on_the_future_road() {
        let mut me = body(Vec3::ZERO, Vec3::Z * 20.0);
        assert!(needs_turnaround(&me, -Vec3::Z * 30.0));
        assert!(!needs_turnaround(&me, Vec3::X * 30.0));
        assert!(!needs_turnaround(&me, Vec3::Z * 30.0));
        let braking = brake_on_future_road(Vec3::Z * 20.0);
        assert_eq!(braking.speed, 0.0);
        assert!(me.velocity.dot(braking.target - me.position) > 0.0);
        me.velocity = Vec3::Z * 10.0;
        assert!(!needs_turnaround(&me, -Vec3::Z * 30.0));
    }
}
