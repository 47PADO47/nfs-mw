//! The original local car environment capture. This is separate from wet-road reflections.
use blackbox_gfx::{Fog, FrameParams, ReflectionQuality, ReflectionScene, RenderBackend, reflection::face_frame};
use glam::Vec3;

use super::{WorldScene, visibility};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CaptureStamp {
    seconds: f32,
    quality: ReflectionQuality,
    origin: Vec3,
    scenery: u64,
    original_shadows: bool,
}

impl WorldScene {
    pub(super) fn update_reflections(&mut self, renderer: &mut dyn RenderBackend) {
        let quality = renderer.graphics().reflections;
        let Some(drive) = &self.drive else {
            renderer.set_reflection_scene(None);
            self.reflection_last = None;
            return;
        };
        if quality == ReflectionQuality::Off || self.car_shading == crate::settings::CarShading::Simple {
            renderer.set_reflection_scene(None);
            self.reflection_last = None;
            return;
        }
        // Cadence and resolution are rewrite choices; all faces share one snapshot. Never accumulate
        // temporal history: at high speed the previous world must not trail across the paint.
        let interval = match quality {
            ReflectionQuality::Low => 1.0 / 15.0,
            ReflectionQuality::Medium => 1.0 / 30.0,
            _ => 0.0,
        };
        let origin = drive.pose().position + Vec3::Z * 0.95;
        let scenery = self.residency.visual_revision();
        if self.reflection_last.is_some_and(|last| {
            last.quality == quality
                && last.scenery == scenery
                && last.original_shadows == self.original_shadows
                && self.clock - last.seconds < interval
                && last.origin.distance_squared(origin) < 4.0
        }) {
            return;
        }
        let template = FrameParams {
            view: glam::Mat4::IDENTITY,
            projection: blackbox_gfx::Projection::Identity,
            camera_position: origin,
            light_dir: self.sun.direction(),
            clear_color: self.fog_colour(),
            fog: Some(Fog { start: self.fog_distance * 0.5, end: self.fog_distance }),
            scattering: self.scattering(),
            surface_light: self.surface_light(),
            camera_cut: true,
        };
        for face in 0..6 {
            let frame = face_frame(origin, face, &template);
            let camera = visibility::Camera {
                light_dir: self.sun.direction().to_array(),
                view_proj: frame.view_proj(),
                position: origin.to_array(),
                forward: frame.view.inverse().transform_vector3(Vec3::NEG_Z).to_array(),
                fov_y_radians: std::f32::consts::FRAC_PI_2,
            };
            visibility::collect_for_height(
                self.residency.placed().filter(|p| self.original_shadows || !p.original_shadow),
                &camera,
                &blackbox_scenery::layout::MOST_WANTED.lod,
                self.residency.props(),
                quality.size() as f32,
                &mut self.reflection_faces[face],
            );
        }
        renderer.set_reflection_scene(Some(ReflectionScene {
            origin,
            quality,
            frame: template,
            faces: self.reflection_faces.each_ref().map(Vec::as_slice),
        }));
        self.reflection_last = Some(CaptureStamp {
            seconds: self.clock,
            quality,
            origin,
            scenery,
            original_shadows: self.original_shadows,
        });
    }
}

#[cfg(test)]
#[path = "reflections_tests.rs"]
mod tests;
