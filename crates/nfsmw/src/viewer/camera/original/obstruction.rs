//! Original outside-view obstruction; see docs/specs/original-camera-obstruction.md.

use glam::Vec3;

use super::Sample;

/// A nearest barrier hit in the camera's Z-up coordinate system.
#[derive(Debug, Clone, Copy)]
pub struct CameraHit {
    pub point: Vec3,
    pub normal: Vec3,
}

/// Retained native clearance. Advance on the camera clock, never per render frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct Obstruction {
    clearance: f32,
}

impl Obstruction {
    /// Clear continuity when the host explicitly cuts or relocates the camera.
    pub fn reset(&mut self) {
        self.clearance = 0.0;
    }

    /// Ordinary-floor convenience for isolated native fixtures.
    #[cfg(test)]
    pub fn step(
        &mut self,
        mode: usize,
        car: Vec3,
        sample: Sample,
        cast: impl FnMut(Vec3, Vec3) -> Option<CameraHit>,
        ground: impl FnMut(Vec3, Vec3, f32) -> Option<f32>,
    ) -> Sample {
        self.step_covered((mode, false), car, sample, cast, ground)
    }

    /// The retained tunnel/overpass state changes only the floor query and ceiling.
    pub fn step_covered(
        &mut self,
        view: (usize, bool),
        car: Vec3,
        sample: Sample,
        mut cast: impl FnMut(Vec3, Vec3) -> Option<CameraHit>,
        mut ground: impl FnMut(Vec3, Vec3, f32) -> Option<f32>,
    ) -> Sample {
        if !(2..=6).contains(&view.0) {
            return sample;
        }
        self.solve(car, sample, view.1, false, &mut cast, &mut ground)
    }

    /// Custom render safety: full clearance without advancing retained history.
    pub fn constrain(
        car: Vec3,
        sample: Sample,
        covered: bool,
        cast: impl FnMut(Vec3, Vec3) -> Option<CameraHit>,
        ground: impl FnMut(Vec3, Vec3, f32) -> Option<f32>,
    ) -> Sample {
        Self::default().solve(car, sample, covered, true, cast, ground)
    }

    fn solve(
        &mut self,
        car: Vec3,
        sample: Sample,
        covered: bool,
        immediate: bool,
        mut cast: impl FnMut(Vec3, Vec3) -> Option<CameraHit>,
        mut ground: impl FnMut(Vec3, Vec3, f32) -> Option<f32>,
    ) -> Sample {
        let delta = car - sample.eye;
        let distance = delta.length();
        if distance < 0.001 {
            return sample;
        }
        let normal = delta / distance;
        let collection_center = (car + sample.eye) * 0.5;
        let collection_radius = ((distance + 1.0) * 0.5).min(24.9);
        let factor = (sample.fov_bangle * 0.000_078_473_77).clamp(1.0, 1.5);
        let side = Vec3::Z.cross(normal) * (factor * distance * 0.44);
        let mut desired = 0.0_f32;
        for direction in [delta + side, delta - side] {
            let end = sample.eye - direction.normalize_or_zero() * 0.9;
            let Some(hit) = cast(car, end) else {
                continue;
            };
            let deficit =
                (car.distance(end) - car.distance(hit.point)) * (1.0 + 0.6 * (1.0 - hit.normal.dot(normal).abs()));
            desired = desired.max(deficit);
        }
        let applied = match immediate {
            true => desired,
            false => (desired + self.clearance) * 0.5,
        };
        self.clearance += desired - applied;
        let applied = match immediate {
            true => applied.clamp(0.0, (distance - 0.05).max(0.0)),
            false => applied.max(0.0),
        };
        let mut eye = sample.eye + normal * applied;
        let query = Vec3::new(eye.x, eye.y, if covered { eye.z } else { eye.z.max(car.z + 0.5) });
        if let Some(height) = ground(query, collection_center, collection_radius) {
            eye.z = eye.z.max(height + 0.25);
            if covered {
                eye.z = eye.z.min(height + 4.5);
            }
        }
        preserve_aim(sample, car, eye)
    }
}

/// Native reconstruction retains the anchor's old projected location and view roll.
pub(super) fn preserve_aim(sample: Sample, car: Vec3, eye: Vec3) -> Sample {
    if eye == sample.eye {
        return sample;
    }
    let forward = (sample.target - sample.eye).normalize_or_zero();
    let right = forward.cross(sample.up).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    let delta = car - sample.eye;
    let depth = -delta.dot(forward);
    if depth >= 0.0 || right == Vec3::ZERO {
        return sample;
    }
    let relative = car - eye;
    let toward_car = relative.normalize_or_zero();
    let new_right = toward_car.cross(up).normalize_or_zero();
    let new_up = new_right.cross(toward_car).normalize_or_zero();
    let scale = relative.length() / depth;
    let target = car + new_right * (delta.dot(right) * scale) + new_up * (delta.dot(up) * scale);
    Sample { eye, target, up, ..sample }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Sample {
        Sample { eye: Vec3::new(-6.0, 0.0, 2.0), target: Vec3::ZERO, up: Vec3::Z, fov_bangle: 16384.0 }
    }

    fn close(actual: Vec3, expected: Vec3) {
        assert!((actual - expected).length() < 0.000_02, "{actual:?} != {expected:?}");
    }

    #[test]
    fn native_two_ray_endpoints_extend_past_the_eye() {
        let mut ends = Vec::new();
        let result = Obstruction::default().step(
            3,
            Vec3::ZERO,
            sample(),
            |from, to| {
                assert_eq!(from, Vec3::ZERO);
                ends.push(to);
                None
            },
            |_, _, _| None,
        );
        assert_eq!(result, sample());
        close(ends[0], Vec3::new(-6.752_317, -0.425_596_48, 2.250_772_2));
        close(ends[1], Vec3::new(-6.752_317, 0.425_596_48, 2.250_772_2));
    }

    #[test]
    fn native_oblique_weight_and_retained_clearance_match() {
        for (normal, correction, history) in [
            (Vec3::X, Vec3::new(1.743_163_2, 0.0, -0.581_054_4), 1.837_455_4),
            (Vec3::Y, Vec3::new(2.705_751, 0.0, -0.901_917_04), 2.852_112),
        ] {
            let mut state = Obstruction::default();
            let result = state.step(
                3,
                Vec3::ZERO,
                sample(),
                |from, to| Some(CameraHit { point: from.lerp(to, 0.5), normal }),
                |_, _, _| None,
            );
            close(result.eye - sample().eye, correction);
            assert!((state.clearance - history).abs() < 0.000_02);
            let old = state.clearance;
            state.step(3, Vec3::ZERO, sample(), |_, _| None, |_, _, _| None);
            assert!((state.clearance - old * 0.5).abs() < 0.000_02);
        }
    }

    #[test]
    fn native_inside_views_bypass_all_obstruction_queries() {
        for mode in [0, 1] {
            let result = Obstruction::default().step(
                mode,
                Vec3::ZERO,
                sample(),
                |_, _| panic!("inside camera must not cast"),
                |_, _, _| panic!("inside camera must not sample a floor"),
            );
            assert_eq!(result, sample());
        }
    }

    #[test]
    fn floor_query_uses_car_height_but_clamps_only_when_a_sample_exists() {
        let mut input = sample();
        input.eye.z = -1.0;
        let result = Obstruction::default().step(
            3,
            Vec3::ZERO,
            input,
            |_, _| None,
            |query, center, radius| {
                assert_eq!(query.z, 0.5);
                close(center, input.eye * 0.5);
                assert!((radius - (input.eye.length() + 1.0) * 0.5).abs() < 0.000_01);
                Some(3.0)
            },
        );
        assert_eq!(result.eye.z, 3.25);
        let open = Obstruction::default().step(3, Vec3::ZERO, input, |_, _| None, |_, _, _| None);
        assert_eq!(open, input);
    }

    #[test]
    fn aim_reconstruction_matches_unchanged_native_matrix_output() {
        let input = Sample { target: Vec3::new(0.0, 0.4, 1.1), ..sample() };
        let moved = preserve_aim(input, Vec3::ZERO, Vec3::new(-3.0, 0.2, 1.5));
        let forward = (moved.target - moved.eye).normalize();
        let right = forward.cross(moved.up).normalize();
        let up = right.cross(forward).normalize();
        close(forward, Vec3::new(0.958_362_4, 0.002_917_138_6, -0.285_539_78));
        close(up, Vec3::new(0.285_496_7, 0.010_360_129, 0.958_323_7));
    }

    #[test]
    fn native_covered_floor_uses_actual_eye_height_and_caps_clearance() {
        let mut state = Obstruction::default();
        let low = Sample { eye: Vec3::new(-6.0, 0.0, -1.0), ..sample() };
        let result = state.step_covered(
            (3, true),
            Vec3::ZERO,
            low,
            |_, _| None,
            |query, _, _| {
                assert_eq!(query.z, -1.0);
                Some(0.0)
            },
        );
        assert_eq!(result.eye.z, 0.25);
        let high = Sample { eye: Vec3::new(-6.0, 0.0, 10.0), ..sample() };
        let result = state.step_covered((3, true), Vec3::ZERO, high, |_, _| None, |_, _, _| Some(3.0));
        assert_eq!(result.eye.z, 7.5);
        let miss = state.step_covered((3, true), Vec3::ZERO, high, |_, _| None, |_, _, _| None);
        assert_eq!(miss, high);
    }
}
