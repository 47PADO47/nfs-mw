//! Orthographic projection of a short, water-filled ellipsoid onto its local view plane.
use blackbox_gfx::EffectVertex;
use glam::Vec3;

pub(super) struct Parcel {
    pub center: Vec3,
    pub along: Vec3,
    pub normal: Vec3,
    /// Semi-axes in the travel, cross-track and surface-normal directions.
    pub radii: Vec3,
}

impl Parcel {
    fn projection(&self, eye: Vec3) -> (Vec3, Vec3, f32) {
        let side = self.normal.cross(self.along);
        let right = eye.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(eye);
        let local = |v: Vec3| Vec3::new(v.dot(self.along), v.dot(side), v.dot(self.normal));
        let x = local(right) * self.radii;
        let y = local(up) * self.radii;
        // Cholesky factor of the projected covariance: these two vectors span
        // the exact projected ellipse even at grazing angles.
        let xx = x.length().max(0.001);
        let yx = x.dot(y) / xx;
        let yy = (y.length_squared() - yx * yx).max(0.000001).sqrt();
        let half_chord = (local(eye) / self.radii).length_recip();
        (right * xx + up * yx, up * yy, half_chord)
    }

    pub fn draw(&self, camera: Vec3, age: f32, seed: f32, density: f32, out: &mut Vec<EffectVertex>) {
        let Some(eye) = (self.center - camera).try_normalize() else { return };
        let (x, y, half_chord) = self.projection(eye);
        // Extinction per metre; leave headroom for overlapping adjacent sections.
        let optical_depth = (half_chord * density / self.radii.y).min(1.8);
        let alpha = (optical_depth * 120.0) as u8;
        if alpha == 0 {
            return;
        }
        for (u, v) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            out.push(EffectVertex {
                position: (self.center + x * u + y * v).to_array(),
                color: [255, 255, 255, alpha],
                uv: [u, v],
                detail: [age, -128.0 - seed],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plume_has_height_from_the_side_and_thickness_controls_the_optical_path() {
        let p = Parcel { center: Vec3::ZERO, along: Vec3::X, normal: Vec3::Z, radii: Vec3::new(0.8, 0.3, 0.2) };
        for (eye, chord, area) in [(Vec3::X, 0.8, 0.06), (Vec3::Y, 0.3, 0.16), (Vec3::Z, 0.2, 0.24)] {
            let (x, y, h) = p.projection(eye);
            assert!((h - chord).abs() < 0.001 && (x.cross(y).length() - area).abs() < 0.001);
        }
    }
}
