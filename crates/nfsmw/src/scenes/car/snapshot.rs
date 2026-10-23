//! A read-only view for reduced captures; only the main frame consumes a camera cut.
use super::*;

impl CarScene {
    pub(crate) fn snapshot(&self, aspect: f32) -> (FrameParams, &[Instance]) {
        let camera: &dyn Camera = match &self.free {
            Some(free) => free,
            None => &self.camera,
        };
        let lens = camera.lens(aspect);
        let mut view = lens.view;
        if self.free.is_none() && self.view_offset != 0.0 {
            // A shift sideways of `s` at the car's distance moves it by `view_offset` of the half-width:
            // the NDC shift is s / (aspect * tan(fov / 2) * distance).
            let half_width = aspect * (lens.fov_y_degrees.to_radians() * 0.5).tan() * self.camera.distance;
            view = Mat4::from_translation(Vec3::X * self.view_offset * half_width) * view;
        }
        let params = FrameParams {
            view,
            projection: lens.projection,
            camera_position: lens.position,
            light_dir: LIGHT_DIR,
            clear_color: [0.18, 0.2, 0.24],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: false,
            shadow_tuning: None,
        };
        (params, &self.instances)
    }
}
