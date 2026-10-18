//! A local pivot for rotating light grids; rebasing preserves their current texel phase.
use glam::{Mat4, Vec2, Vec3};

#[derive(Default)]
pub(super) struct Grid {
    origin: Option<Vec3>,
    phase: [Vec2; 4],
    #[cfg(test)]
    world_origin: bool,
}

impl Grid {
    pub fn follow(&mut self, camera: Vec3) -> Vec3 {
        #[cfg(test)]
        if self.world_origin {
            return Vec3::ZERO;
        }
        let Some(origin) = self.origin else {
            self.origin = Some(camera);
            return Vec3::ZERO;
        };
        if camera.distance_squared(origin) <= 128.0 * 128.0 {
            return Vec3::ZERO;
        }
        self.origin = Some(camera);
        camera - origin
    }

    #[cfg(test)]
    pub fn world_origin() -> Self {
        Self { origin: Some(Vec3::ZERO), world_origin: true, ..Default::default() }
    }

    pub fn snap(&mut self, rotation: Mat4, center: Vec3, texel: f32, cascade: usize, rebase: Vec3) -> Vec3 {
        let origin = self.origin.expect("grid follows camera before fitting");
        let phase = &mut self.phase[cascade];
        let shift = rotation.transform_vector3(rebase).truncate();
        *phase = (*phase + shift).rem_euclid(Vec2::splat(texel));
        let relative = rotation.transform_vector3(center - origin);
        let pivot = rotation.transform_vector3(origin);
        let snapped = ((relative.truncate() + *phase) / texel).round() * texel - *phase;
        (snapped + pivot.truncate()).extend(((relative.z + pivot.z) / texel).round() * texel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebase_keeps_the_existing_projection_phase_for_each_cascade() {
        let rotation = Mat4::from_rotation_x(0.4) * Mat4::from_rotation_z(0.7);
        let mut grid = Grid::default();
        grid.follow(Vec3::new(2151.0, 1399.0, 148.7));
        let center = Vec3::new(2284.0, 1394.0, 150.0);
        let before: Vec<_> = [0.014, 0.05, 0.2, 0.6]
            .into_iter()
            .enumerate()
            .map(|(i, texel)| grid.snap(rotation, center, texel, i, Vec3::ZERO))
            .collect();
        let shift = grid.follow(center);
        assert!(shift.length() > 128.0);
        for (i, texel) in [0.014, 0.05, 0.2, 0.6].into_iter().enumerate() {
            let after = grid.snap(rotation, center, texel, i, shift);
            assert!((before[i] - after).length() < 0.001, "cascade {i}: rebase moved the grid");
        }
    }

    #[test]
    fn moving_sun_does_not_spin_the_nearby_grid_about_the_city_origin() {
        let camera = Vec3::new(2151.0, 1399.0, 148.7);
        let center = camera + Vec3::new(0.0, 12.0, -2.0);
        let texel = 0.014;
        let mut grid = Grid::default();
        grid.follow(camera);
        let mut local_error = Vec::new();
        let mut old_error = Vec::new();
        for i in 0..120 {
            let rotation = Mat4::from_rotation_x(0.25) * Mat4::from_rotation_z(0.4 + i as f32 * 0.00003);
            let light = rotation.transform_point3(center);
            let local = grid.snap(rotation, center, texel, 0, Vec3::ZERO);
            local_error.push(local.truncate() - light.truncate());
            old_error.push((light.truncate() / texel).round() * texel - light.truncate());
        }
        let changes = |errors: &[Vec2]| errors.windows(2).map(|v| (v[1] - v[0]).length_squared()).sum::<f32>();
        assert!(
            changes(&local_error) < changes(&old_error) * 0.25,
            "local pivot must reduce snapping phase changes under a moving sun"
        );
    }
}
