//! The sun the world is lit by.

use glam::Vec3;

/// The direction the sun's light travels.
pub(super) const DIRECTION: Vec3 = Vec3::new(-0.35, -0.45, -1.0);

/// The direction from the cars towards the sun, for their light rig.
pub(super) fn to_sun() -> Vec3 {
    -DIRECTION
}
