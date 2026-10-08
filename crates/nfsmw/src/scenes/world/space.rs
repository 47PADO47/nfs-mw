//! The two coordinate spaces: the world and the models are Z-up (x forward, y left, z up), while
//! collision and physics use x right, y up, z forward (`docs/formats/collision.md`). The game
//! converts with `(x, y, z) -> (-y, z, x)`.

use blackbox_collision::CollisionWorld;
use glam::Vec3;

/// Render space to physics space.
pub fn to_physics(p: Vec3) -> [f32; 3] {
    [-p.y, p.z, p.x]
}

/// The first surface below `(x, y, from_z)` (render space) down to `to_z`, as a height.
pub fn ground_below(collision: &CollisionWorld, x: f32, y: f32, from_z: f32, to_z: f32) -> Option<f32> {
    let [px, _, pz] = to_physics(Vec3::new(x, y, 0.0));
    collision.ground(px, pz, from_z, to_z).map(|hit| hit.point[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_convert_axis_by_axis() {
        // Forward (+x in the world) is +z in physics, up stays up.
        assert_eq!(to_physics(Vec3::X), [0.0, 0.0, 1.0]);
        assert_eq!(to_physics(Vec3::Z), [0.0, 1.0, 0.0]);
        // Left (+y in the world) is -x in physics.
        assert_eq!(to_physics(Vec3::Y), [-1.0, 0.0, 0.0]);
    }
}
