//! Short circular attachment followed by tangential release, drag and gravity.
use glam::Vec3;

#[derive(Clone, Copy)]
pub(in crate::scenes::world::vehicle_effects) struct Tread {
    pub forward: Vec3,
    pub radius: f32,
    pub spin: f32,
}

impl Tread {
    /// Offset from the world-space contact at emission. Positive wheel spin rolls forward.
    pub(super) fn displacement(self, normal: Vec3, velocity: Vec3, age: f32, release: f32) -> Vec3 {
        let omega = self.spin.abs();
        if omega < 1.0 {
            return Vec3::ZERO;
        }
        let along = self.forward * self.spin.signum();
        let carried = age.min(release / omega);
        let (sin, cos) = (omega * carried).sin_cos();
        let attached = velocity * carried + self.radius * (-along * sin + normal * (1.0 - cos));
        let flight = age - carried;
        if flight <= 0.0 {
            return attached;
        }
        let rim_speed = self.radius * omega;
        let tangent = velocity + rim_speed * (-along * cos + normal * sin);
        // Integrate a decaying release impulse. Faster drops lose more momentum to air.
        let drag = 6.0 + 0.62 * rim_speed;
        attached + tangent * (-(-drag * flight).exp_m1() / drag) - Vec3::Z * (4.905 * flight * flight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bead_leaves_continuously_and_returns_to_ground() {
        for sign in [-1.0, 1.0] {
            let tire = Tread { forward: Vec3::X, radius: 0.35, spin: sign * 60.0 };
            let at = |t| tire.displacement(Vec3::Z, Vec3::X * sign * 21.0, t, 0.6);
            assert_eq!(at(0.0), Vec3::ZERO);
            let release = 0.01;
            let before = (at(release) - at(release - 0.00001)) / 0.00001;
            let after = (at(release + 0.00001) - at(release)) / 0.00001;
            assert!(before.distance(after) < 0.12);
            assert!(at(0.09).z > at(release).z && at(0.46).z < 0.0);
        }
    }
}
