//! Hit-testing a pointer against the buttons of a package, so a click can focus the button under it and press it
//! the way accept does (the mouse side of spec `docs/specs/feng-input.md`).

use glam::{Mat2, Vec2};

use super::{ObjectRef, PackageId, Runtime};

impl Runtime {
    /// The visible button under a point of the 640 x 480 screen (origin at the centre, y down), the nearest one when
    /// buttons overlap. A hidden button, or one whose parent is hidden, is never hit.
    pub fn button_at(&self, package: PackageId, point: Vec2) -> Option<ObjectRef> {
        let p = self.running(package)?;
        let tree = self.tree(package);
        let mut best: Option<(usize, f32)> = None;
        for node in &tree.nodes {
            if !node.visible || !p.buttons.contains(&node.index) || !contains(&node.world, point) {
                continue;
            }
            // Larger depth is farther, so the smallest one is on top.
            if best.is_none_or(|(_, z)| node.z < z) {
                best = Some((node.index, node.z));
            }
        }
        let (index, _) = best?;
        Some(ObjectRef { package, index })
    }

    /// Makes a button the current one, as moving the focus to it does, so the next accept presses it.
    pub fn focus_button(&mut self, button: ObjectRef) {
        self.set_focus_index(button.package, Some(button.index), true);
    }
}

/// Whether `point` lies on the unit quad that `world` maps to the screen (the quad spans -0.5 .. 0.5).
fn contains(world: &glam::Mat4, point: Vec2) -> bool {
    let axes = Mat2::from_cols(world.x_axis.truncate().truncate(), world.y_axis.truncate().truncate());
    let origin = world.w_axis.truncate().truncate();
    let local = axes.inverse() * (point - origin);
    local.x.abs() <= 0.5 && local.y.abs() <= 0.5
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3};

    use super::*;

    #[test]
    fn a_point_is_inside_the_quad_it_is_scaled_and_moved_to() {
        // A 100 x 40 button centred at (200, -50).
        let world = Mat4::from_scale_rotation_translation(Vec3::new(100.0, 40.0, 1.0), glam::Quat::IDENTITY, Vec3::new(200.0, -50.0, 0.0));
        assert!(contains(&world, Vec2::new(200.0, -50.0)));
        assert!(contains(&world, Vec2::new(249.0, -31.0)));
        assert!(!contains(&world, Vec2::new(251.0, -50.0)), "just right of the edge");
        assert!(!contains(&world, Vec2::new(200.0, -29.0)), "just below the edge");
    }

    #[test]
    fn a_rotated_quad_is_hit_on_its_rotated_shape() {
        let world = Mat4::from_scale_rotation_translation(
            Vec3::new(100.0, 10.0, 1.0),
            glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            Vec3::ZERO,
        );
        assert!(contains(&world, Vec2::new(0.0, 45.0)), "the long side now points up");
        assert!(!contains(&world, Vec2::new(45.0, 0.0)), "and not right");
    }

    #[test]
    fn a_degenerate_quad_is_never_hit() {
        let world = Mat4::from_scale(Vec3::new(0.0, 0.0, 1.0));
        assert!(!contains(&world, Vec2::ZERO));
    }
}
