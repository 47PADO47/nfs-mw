//! The camera the world is drawn from, for the HUD's world-space markers: they project through it, so they stay on
//! their places in the world (`scenes/world/camera_choice.rs`).

/// The gameplay camera of the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudCamera {
    /// World (render space) to clip space, column major as `glam::Mat4::to_cols_array_2d` gives it. The projection is
    /// reverse-Z with no far plane.
    pub view_proj: [[f32; 4]; 4],
    /// The size of the view the matrix maps onto, in the HUD's logical pixels (width, height). The HUD plugin fills it
    /// from the window; zero until then.
    pub viewport: [f32; 2],
}

impl HudCamera {
    /// Where a render-space point lands in the viewport, in logical pixels from the top left, with its clip depth; `None`
    /// for a point behind the camera (or on its plane).
    #[allow(dead_code)] // read by the Custom HUD's world markers, not built yet
    pub fn project(&self, point: [f32; 3]) -> Option<([f32; 2], f32)> {
        let m = glam::Mat4::from_cols_array_2d(&self.view_proj);
        let clip = m * glam::Vec4::new(point[0], point[1], point[2], 1.0);
        if clip.w <= f32::EPSILON {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let [width, height] = self.viewport;
        Some(([(ndc.x + 1.0) * 0.5 * width, (1.0 - ndc.y) * 0.5 * height], ndc.z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::camera::Lens;
    use glam::Vec3;

    /// The game's own lens at the origin, looking along +x (render space: x forward, y left, z up).
    fn camera() -> HudCamera {
        let lens = Lens::look_at(Vec3::ZERO, Vec3::X, 60.0, 0.3, 16.0 / 9.0);
        HudCamera { view_proj: lens.view_proj().to_cols_array_2d(), viewport: [1600.0, 900.0] }
    }

    #[test]
    fn a_point_straight_ahead_lands_in_the_middle() {
        let (at, _) = camera().project([50.0, 0.0, 0.0]).unwrap();
        assert!((at[0] - 800.0).abs() < 1e-3 && (at[1] - 450.0).abs() < 1e-3, "{at:?}");
    }

    #[test]
    fn a_point_to_the_left_and_above_lands_left_and_up() {
        // Render space: y is left, z is up.
        let (at, _) = camera().project([50.0, 5.0, 5.0]).unwrap();
        assert!(at[0] < 800.0 && at[1] < 450.0, "{at:?}");
    }

    #[test]
    fn a_point_behind_the_camera_does_not_project() {
        assert_eq!(camera().project([-10.0, 0.0, 0.0]), None);
    }
}
