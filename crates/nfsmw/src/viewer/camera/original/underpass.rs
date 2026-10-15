//! Original vehicle-underpass camera pulse; see original-camera-obstruction spec.

use glam::Vec3;

use super::{Sample, vehicle_avoidance::Avoidable};

// Authored 0.707 threshold, deliberately less precise than the geometric constant.
const CROSSING_COSINE: f32 = f32::from_bits(0x3f34_fdf4);

#[derive(Debug, Clone, Copy)]
pub struct VehicleBounds {
    pub world_id: u32,
    pub body: Avoidable,
    /// Native rigid-body broad radius, not the camera-inflated roof radius.
    pub radius: f32,
    pub active: bool,
    pub loading: bool,
}

/// Native predicate uses the previous camera direction and the predicted geometry anchor.
pub fn under_vehicle(
    world_id: u32,
    car: Vec3,
    velocity: Vec3,
    camera_forward: Vec3,
    vehicles: &[VehicleBounds],
) -> bool {
    let predicted = car + velocity * 0.05;
    for vehicle in vehicles {
        if !vehicle.active || vehicle.loading || vehicle.world_id == world_id {
            continue;
        }
        let forward = vehicle.body.transform.x_axis.truncate();
        if forward.dot(camera_forward).abs() >= CROSSING_COSINE {
            continue;
        }
        let center = vehicle.body.transform.w_axis.truncate();
        if predicted.distance(center) >= vehicle.radius {
            continue;
        }
        let local = vehicle.body.transform.inverse().transform_point3(predicted);
        let half = vehicle.body.half_extents;
        if local.x.abs() < half.x && local.y.abs() < half.y {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, Copy)]
pub struct Underpass {
    elapsed: f64,
}

impl Default for Underpass {
    fn default() -> Self {
        Self { elapsed: 1.0 }
    }
}

impl Underpass {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advance once per camera tick. Detection refreshes the timestamp in every POV.
    pub fn advance(&mut self, under: bool, dt: f32) {
        self.elapsed = match under {
            true => 0.0,
            false => self.elapsed + f64::from(dt),
        };
    }

    /// Apply before feedback/noise and static clearance. The boolean suppresses body elevation.
    pub fn apply(&self, mode: usize, mut sample: Sample) -> (Sample, bool) {
        if !(2..=6).contains(&mode) {
            return (sample, false);
        }
        // The original subtracts integer 4 kHz timestamps, then evaluates the
        // authored float constants in the x87 register before storing phase.
        let ticks = (self.elapsed * 4000.0).round();
        let phase = ((f64::from(1.2_f32) - ticks * f64::from(0.00025_f32) * 1.25) as f32).clamp(0.0, 1.2);
        let strength = match phase > 1.0 {
            true => 2.0 - phase,
            false => phase,
        };
        sample.eye.z -= 0.9 * strength;
        (sample, strength > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Quat};

    use super::*;

    fn vehicle(position: Vec3, yaw: f32, radius: f32) -> VehicleBounds {
        VehicleBounds {
            world_id: 2,
            body: Avoidable {
                transform: Mat4::from_rotation_translation(Quat::from_rotation_z(yaw), position),
                half_extents: Vec3::new(2.0, 1.0, 0.7),
                velocity: Vec3::ZERO,
            },
            radius,
            active: true,
            loading: false,
        }
    }

    #[test]
    fn underpass_predicate_matches_native_orientation_radius_and_prediction() {
        let perpendicular = std::f32::consts::FRAC_PI_2;
        for (position, yaw, radius, velocity, expected) in [
            (Vec3::ZERO, 0.0, 3.0, Vec3::ZERO, false),
            (Vec3::ZERO, perpendicular, 1.0, Vec3::ZERO, true),
            (Vec3::Z * 2.0, perpendicular, 1.0, Vec3::ZERO, false),
            (Vec3::Z * 2.0, perpendicular, 3.0, Vec3::ZERO, true),
            (Vec3::X * 2.0, perpendicular, 3.0, Vec3::ZERO, false),
            (Vec3::Y * 2.0, perpendicular, 3.0, Vec3::ZERO, false),
            (Vec3::X * 1.8, perpendicular, 3.0, Vec3::X * 20.0, true),
        ] {
            assert_eq!(under_vehicle(1, Vec3::ZERO, velocity, Vec3::X, &[vehicle(position, yaw, radius)]), expected);
        }
    }

    #[test]
    fn inactive_loading_and_anchor_vehicles_do_not_trigger_a_pulse() {
        let base = vehicle(Vec3::ZERO, std::f32::consts::FRAC_PI_2, 3.0);
        for candidate in [
            VehicleBounds { active: false, ..base },
            VehicleBounds { loading: true, ..base },
            VehicleBounds { world_id: 1, ..base },
        ] {
            assert!(!under_vehicle(1, Vec3::ZERO, Vec3::ZERO, Vec3::X, &[candidate]));
        }
    }

    #[test]
    fn pulse_is_retained_and_does_not_move_inside_views_or_the_target() {
        let sample = Sample { eye: Vec3::new(-6.0, 0.0, 2.0), target: Vec3::ZERO, up: Vec3::Z, fov_bangle: 16384.0 };
        let mut pulse = Underpass::default();
        assert_eq!(pulse.apply(3, sample), (sample, false));
        pulse.advance(true, 1.0 / 60.0);
        assert_eq!(pulse.apply(1, sample), (sample, false));
        let (held, suppress) = pulse.apply(3, sample);
        assert!(suppress);
        assert!((held.eye.z - 1.28).abs() < 0.000_01);
        assert_eq!(held.target, sample.target);
        pulse.advance(false, 0.16);
        assert!((pulse.apply(3, sample).0.eye.z - 1.1).abs() < 0.000_01);
        pulse.advance(false, 0.81);
        assert_eq!(pulse.apply(3, sample), (sample, false));
        for (elapsed, expected, suppressed) in [
            (0.0, 1.280_000_1, true),
            (0.04, 1.235_000_1, true),
            (0.16, 1.1, true),
            (0.5, 1.482_5, true),
            (0.9, 1.932_5, true),
            (0.9595, 1.999_437_5, true),
            (0.95975, 1.999_718_8, true),
            (0.96, 2.0, false),
            (0.96025, 2.0, false),
            (1.2, 2.0, false),
        ] {
            let (result, active) = Underpass { elapsed }.apply(3, sample);
            assert!((result.eye.z - expected).abs() < 0.000_01);
            assert_eq!(active, suppressed);
        }
    }
}
