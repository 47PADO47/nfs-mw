//! A lens that is part way between two others, for the cameras that hand over smoothly (a bike's helmet camera and the chase camera
//! on its thrown rider, `docs/specs/ride6-bike-cameras.md` section 8).

use blackbox_gfx::Projection;
use glam::{Mat4, Quat, Vec3};

use super::Lens;

impl Lens {
    /// The lens `share` (0 to 1) of the way from `self` to `other`: the place and the field of view blend linearly, the turn
    /// (the roll included) on the short arc. The projection is `self`'s with the blended field of view and the nearer near plane.
    pub fn mix(&self, other: &Lens, share: f32) -> Lens {
        let share = share.clamp(0.0, 1.0);
        if share <= 0.0 {
            return *self;
        }
        if share >= 1.0 {
            return *other;
        }
        let (a, b) = (turn_of(self), turn_of(other));
        let turn = a.slerp(b, share);
        let position = self.position.lerp(other.position, share);
        let fov_y_degrees = self.fov_y_degrees + (other.fov_y_degrees - self.fov_y_degrees) * share;
        let projection = match (self.projection, other.projection) {
            (
                Projection::PerspectiveInfiniteReverse { aspect, near, .. },
                Projection::PerspectiveInfiniteReverse { near: other_near, .. },
            ) => Projection::PerspectiveInfiniteReverse {
                fov_y: fov_y_degrees.to_radians(),
                aspect,
                near: near.min(other_near),
            },
            _ => self.projection,
        };
        Lens {
            view: Mat4::from_rotation_translation(turn, position).inverse(),
            projection,
            position,
            forward: turn * Vec3::NEG_Z,
            fov_y_degrees,
        }
    }
}

/// The orientation of a lens' camera in the world (the view matrix is its inverse).
fn turn_of(lens: &Lens) -> Quat {
    let (_, turn, _) = lens.view.inverse().to_scale_rotation_translation();
    turn.normalize()
}

#[cfg(test)]
#[path = "lens_mix_tests.rs"]
mod tests;
