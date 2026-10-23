//! The menu car uses the same local-reflection quality and resolution as a driving car. Its stationary room
//! is captured once, without the reflective car itself, and recaptured when quality or resolution changes.

use blackbox_gfx::reflection::{ALL_FACES, PLANAR_MAX};
use blackbox_gfx::{ReflectionQuality, ReflectionScene, RenderBackend};
use glam::Vec3;

use super::{MenuScene, flat_frame};
use crate::settings::CarShading;

impl MenuScene {
    pub(super) fn update_reflections(&mut self, renderer: &mut dyn RenderBackend) {
        let quality = renderer.graphics().reflections;
        let Some(room) = self
            .room
            .as_ref()
            .filter(|_| self.car.is_some() && self.shading != CarShading::Simple && quality != ReflectionQuality::Off)
        else {
            renderer.set_reflection_scene(None);
            self.reflection = None;
            return;
        };
        let size =
            renderer.graphics().reflection_resolution.cube_face(quality.size(), renderer.surface_size()[1], PLANAR_MAX);
        let stamp = (quality, size);
        let refresh = match self.reflection == Some(stamp) {
            true => 0,
            false => ALL_FACES,
        };
        let origin = Vec3::Z * 0.95;
        renderer.set_reflection_scene(Some(ReflectionScene {
            sun_shadows: false,
            fade_distance: None,
            origin,
            quality,
            frame: blackbox_gfx::FrameParams {
                camera_position: origin,
                light_dir: Vec3::new(-0.4, -0.3, -1.0),
                clear_color: [0.18, 0.2, 0.24],
                ..flat_frame()
            },
            faces: [room.instances(); 6],
            refresh,
        }));
        self.reflection = Some(stamp);
    }
}
