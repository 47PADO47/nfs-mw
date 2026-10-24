//! Car-only floor capture; the room retains its authored blended order.
use super::super::camera_pose::CameraPose;
use super::{MenuScene, Part, car_part};
use crate::settings::CarShading;
use blackbox_gfx::{PlanarReflectionScene, ReflectionQuality, RenderBackend};
use glam::{Mat4, Vec3};

impl MenuScene {
    pub(super) fn update_floor_reflection(&mut self, renderer: &mut dyn RenderBackend) {
        self.update_floor_reflection_from(renderer, None);
    }

    pub(super) fn update_floor_reflection_from(
        &mut self,
        renderer: &mut dyn RenderBackend,
        camera: Option<CameraPose>,
    ) {
        let quality = renderer.graphics().reflections;
        let receiver = self.room.as_ref().and_then(|room| room.floor_texture());
        let Some(receiver) = receiver.filter(|_| {
            self.car.is_some()
                && renderer.capabilities().projected_reflections
                && quality != ReflectionQuality::Off
                && self.shading != CarShading::Simple
        }) else {
            renderer.set_planar_reflection_scene(None);
            return;
        };
        let [width, height] = renderer.surface_size();
        if width == 0 || height == 0 {
            return;
        }
        let edge = renderer.graphics().reflection_resolution.cube_face(quality.size(), height, 2048);
        let size = [((edge as f32 * width as f32 / height as f32).round() as u32).max(1), edge.max(1)];
        let aspect = width as f32 / height as f32;
        let (frame, parts) = self.car.as_ref().expect("car checked above").snapshot(aspect);
        let camera = camera.or_else(|| self.rig.pose().filter(|_| self.look.camera_intro().is_some()));
        let frame = reflected_frame(frame, camera, aspect);
        let car: Vec<_> = parts.iter().filter(|part| matches!(car_part(part), Part::Car)).copied().collect();
        renderer.set_planar_reflection_scene(Some(PlanarReflectionScene {
            frame,
            size,
            instances: &car,
            clip_plane: [0.0, 0.0, 1.0, 0.0],
            receivers: &[receiver],
        }));
    }
}

fn reflected_frame(
    mut frame: blackbox_gfx::FrameParams,
    camera: Option<CameraPose>,
    aspect: f32,
) -> blackbox_gfx::FrameParams {
    if let Some(pose) = camera {
        let lens = pose.lens(aspect);
        (frame.view, frame.projection, frame.camera_position) = (lens.view, lens.projection, lens.position);
    }
    let mirror = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
    frame.view *= mirror;
    frame.camera_position = mirror.transform_point3(frame.camera_position);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::camera_pose::SAFE_HOUSE;

    #[test]
    fn reflected_floor_uses_the_effective_inspection_lens_instead_of_the_screen_camera() {
        let pose =
            CameraPose { yaw: 117.0, pitch: 101.0, distance: 4.8, fov: 45.0, target: Vec3::Z * 0.75, ..SAFE_HOUSE };
        let aspect = 16.0 / 9.0;
        let frame = reflected_frame(super::super::flat_frame(), Some(pose), aspect);
        let mirror = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
        assert!((frame.camera_position - mirror.transform_point3(pose.eye())).length() < 1e-6);
        assert!(frame.view.abs_diff_eq(pose.lens(aspect).view * mirror, 1e-6));
        let actual = frame.projection.matrix();
        let expected = pose.lens(aspect).projection.matrix();
        assert!(actual.abs_diff_eq(expected, 1e-6));
    }
}
