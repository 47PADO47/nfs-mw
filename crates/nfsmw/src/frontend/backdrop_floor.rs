//! Car-only floor capture; the room retains its authored blended order.
use super::{MenuScene, Part, car_part};
use crate::settings::CarShading;
use blackbox_gfx::{PlanarReflectionScene, ReflectionQuality, RenderBackend};
use glam::{Mat4, Vec3};

impl MenuScene {
    pub(super) fn update_floor_reflection(&mut self, renderer: &mut dyn RenderBackend) {
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
        let (mut frame, parts) = self.car.as_ref().expect("car checked above").snapshot(aspect);
        if let Some(pose) = self.rig.pose().filter(|_| self.look.camera_intro().is_some()) {
            let lens = pose.lens(aspect);
            (frame.view, frame.projection, frame.camera_position) = (lens.view, lens.projection, lens.position);
        }
        let car: Vec<_> = parts.iter().filter(|part| matches!(car_part(part), Part::Car)).copied().collect();
        let mirror = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
        frame.view *= mirror;
        frame.camera_position = mirror.transform_point3(frame.camera_position);
        renderer.set_planar_reflection_scene(Some(PlanarReflectionScene {
            frame,
            size,
            instances: &car,
            clip_plane: [0.0, 0.0, 1.0, 0.0],
            receivers: &[receiver],
        }));
    }
}
