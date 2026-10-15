//! Avoidable-body camera response; see docs/specs/original-camera-obstruction.md.

use glam::{Mat4, Vec3};

use super::{Sample, obstruction::preserve_aim};

/// A body registered for camera avoidance, in list order and Z-up render axes.
#[derive(Debug, Clone, Copy)]
pub struct Avoidable {
    /// Local forward/right/up axes followed by the body center.
    pub transform: Mat4,
    /// Half dimensions along local forward/right/up.
    pub half_extents: Vec3,
    pub velocity: Vec3,
}

/// Retained secondary eye elevation; advance on the stable camera clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct Avoidance {
    adjustment: f32,
    saved: f32,
    previous_row: Vec3,
}

impl Avoidance {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Call after static clearance for outside views, unless the displacement pulse suppresses it.
    pub fn step(&mut self, mode: usize, car: Vec3, velocity: Vec3, sample: Sample, bodies: &[Avoidable]) -> Sample {
        if !(2..=6).contains(&mode) {
            return sample;
        }
        let mut eye = Self::constrain(car, sample, bodies).eye;
        let mut desired = eye.z - sample.eye.z;
        let forward = (sample.target - sample.eye).normalize_or_zero();
        let right = forward.cross(sample.up).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let row = Vec3::new(right.x, -up.x, forward.x);
        let same_direction = row.dot(self.previous_row) > 0.9;
        self.previous_row = row;
        if velocity.truncate().length() < 1.0 && same_direction && desired < self.saved {
            desired = self.saved;
        }
        self.saved = desired;
        let applied = (desired + self.adjustment) * 0.5;
        self.adjustment += desired - applied;
        eye.z = sample.eye.z + applied;
        preserve_aim(sample, car, eye)
    }
    /// Immediate custom-camera roof clearance without retained state.
    pub fn constrain(car: Vec3, sample: Sample, bodies: &[Avoidable]) -> Sample {
        let mut eye = sample.eye;
        for _ in 0..16 {
            let raise = under_body(eye, bodies);
            if raise <= 0.0 {
                break;
            }
            eye.z += raise;
        }
        preserve_aim(sample, car, eye)
    }
}

/// First eligible body's rounded roof clearance. List order is significant.
fn under_body(eye: Vec3, bodies: &[Avoidable]) -> f32 {
    for body in bodies {
        let half = body.half_extents + Vec3::splat(0.85);
        let delta = body.transform.w_axis.truncate() - eye;
        let radius_squared = half.x * half.x + half.y * half.y;
        if delta.truncate().length_squared() >= radius_squared || delta.z.abs() >= half.z * 2.0 {
            continue;
        }
        let mut local = body.transform.inverse().transform_point3(eye);
        let fraction = (local.x / half.x).powi(4) + (local.y / half.y).powi(4);
        if fraction >= 1.0 {
            continue;
        }
        let roof = (1.0 - fraction).sqrt().sqrt() * half.z;
        if local.z >= roof {
            continue;
        }
        local.z = roof;
        let raise = body.transform.transform_point3(local).z - eye.z;
        if raise <= radius_squared.sqrt() {
            return raise;
        }
        return 0.0;
    }
    0.0
}

/// Native avoidance contribution to the follow up-vector target.
pub fn up_influence(car: Vec3, velocity: Vec3, bodies: &[Avoidable]) -> Vec3 {
    if velocity.length() <= 5.0 {
        return Vec3::ZERO;
    }
    let forward = velocity.normalize_or_zero();
    let mut influence = Vec3::ZERO;
    for body in bodies {
        let delta = body.transform.w_axis.truncate() - car;
        let distance_squared = delta.length_squared();
        if distance_squared <= 0.8_f32 * 0.8 || distance_squared >= 49.0 {
            continue;
        }
        let direction = delta.normalize_or_zero();
        if direction.dot(forward) <= 0.5 {
            continue;
        }
        let attenuation = (4.84 / distance_squared).clamp(0.0, 1.0);
        let relative = ((velocity.distance(body.velocity) - 15.0) / 85.0).clamp(0.0, 1.0);
        let opposing = (-forward.dot(body.velocity.normalize_or_zero())).max(0.0);
        influence += direction * (attenuation * (relative + opposing));
    }
    influence
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(position: Vec3, velocity: Vec3) -> Avoidable {
        Avoidable { transform: Mat4::from_translation(position), half_extents: Vec3::new(2.0, 1.0, 0.7), velocity }
    }

    #[test]
    fn rounded_roof_matches_unchanged_native_body_calls() {
        let bodies = [body(Vec3::ZERO, Vec3::ZERO)];
        for (eye, expected) in [
            (Vec3::ZERO, 1.55),
            (Vec3::X, 1.544_092_9),
            (Vec3::Y, 1.515_803_3),
            (Vec3::X * 4.0, 0.0),
            (Vec3::Z * 2.0, 0.0),
            (Vec3::Z * -2.0, 0.0),
        ] {
            assert!((under_body(eye, &bodies) - expected).abs() < 0.000_01);
        }
    }

    #[test]
    fn native_up_influence_weighs_closing_speed_and_opposition() {
        let car_velocity = Vec3::X * 20.0;
        for (position, velocity, expected) in [
            (Vec3::X * 3.0, Vec3::ZERO, Vec3::X * 0.031_633_988),
            (Vec3::X * 3.0, Vec3::X * -30.0, Vec3::X * 0.759_215_7),
            (Vec3::new(3.0, 2.0, 0.0), Vec3::ZERO, Vec3::new(0.018_222_28, 0.012_148_187, 0.0)),
            (Vec3::new(3.0, 2.0, 0.0), Vec3::X * -30.0, Vec3::new(0.437_334_75, 0.291_556_48, 0.0)),
            (Vec3::X * -3.0, Vec3::X * -30.0, Vec3::ZERO),
            (Vec3::X * 0.7, Vec3::X * -30.0, Vec3::ZERO),
            (Vec3::X * 7.0, Vec3::X * -30.0, Vec3::ZERO),
            (Vec3::X * 3.0, car_velocity, Vec3::ZERO),
        ] {
            let result = up_influence(Vec3::ZERO, car_velocity, &[body(position, velocity)]);
            assert!((result - expected).length() < 0.000_01, "{result:?} != {expected:?}");
        }
        assert_eq!(up_influence(Vec3::ZERO, Vec3::X * 5.0, &[body(Vec3::X * 3.0, Vec3::ZERO)]), Vec3::ZERO);
    }

    #[test]
    fn stationary_clearance_holds_until_the_view_direction_changes() {
        let sample = Sample {
            eye: Vec3::new(0.0, 0.0, 0.5),
            target: Vec3::new(10.0, 0.0, 0.5),
            up: Vec3::Z,
            fov_bangle: 16384.0,
        };
        // Anchor ahead of the eye, so the native reconstruction has positive view depth.
        let car = Vec3::X * 6.0;
        let mut state = Avoidance::default();
        state.step(3, car, Vec3::ZERO, sample, &[body(Vec3::ZERO, Vec3::ZERO)]);
        let desired = state.saved;
        state.step(3, car, Vec3::ZERO, sample, &[]);
        assert_eq!(state.saved, desired);
        let reversed = Sample { target: Vec3::X * -10.0, ..sample };
        state.step(3, car, Vec3::ZERO, reversed, &[]);
        assert_eq!(state.saved, 0.0);
    }
}
