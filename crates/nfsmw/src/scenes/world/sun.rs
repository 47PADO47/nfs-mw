//! The sun the world is lit by.

use blackbox_render::Renderer;
use glam::Vec3;

use crate::scenes::car::lighting;

/// The direction the sun's light travels.
pub(super) const DIRECTION: Vec3 = Vec3::new(-0.35, -0.45, -1.0);

/// Light the cars (glossy shading) by the sun.
pub(super) fn light_cars(renderer: &mut Renderer) {
    renderer.set_lighting_rig(&lighting::rig(-DIRECTION));
}
