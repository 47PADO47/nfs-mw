use glam::Vec3;

/// Principal moments of a solid box with half extents `half` and mass `mass`, in the body frame
/// (x right, y up, z forward): `(Ix, Iy, Iz)` are the moments about the right, up and forward axes.
pub fn box_inertia(mass: f32, half: Vec3) -> Vec3 {
    let (w, h, l) = (2.0 * half.x, 2.0 * half.y, 2.0 * half.z);
    Vec3::new(h * h + l * l, w * w + l * l, w * w + h * h) * (mass / 12.0)
}

/// The inverse of a diagonal inertia tensor; a moment at or below epsilon is left as-is (0 inverse).
pub fn inverse_diagonal(inertia: Vec3) -> Vec3 {
    let inv = |i: f32| if i > f32::EPSILON { 1.0 / i } else { 0.0 };
    Vec3::new(inv(inertia.x), inv(inertia.y), inv(inertia.z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_cube_inertia() {
        // A cube of side 1 and mass 6 has I = m s^2 / 6 = 1 about every axis.
        let i = box_inertia(6.0, Vec3::splat(0.5));
        assert!((i - Vec3::ONE).abs().max_element() < 1e-6);
    }

    #[test]
    fn inverse_skips_zero() {
        assert_eq!(inverse_diagonal(Vec3::new(2.0, 0.0, 4.0)), Vec3::new(0.5, 0.0, 0.25));
    }
}
