//! The original local car environment capture. This is separate from wet-road reflections.
//!
//! The cube's reach, scenery detail and content come from the requested settings (`reflection_distance`,
//! `reflection_lod`, `reflection_content`; docs/specs/hd-reflections.md, sections 6 to 8).
use blackbox_gfx::reflection::content::ReflectionContent;
use blackbox_gfx::reflection::distance::{ReflectionDistance, within};
use blackbox_gfx::reflection::lod::ReflectionLod;
use blackbox_gfx::reflection::{PLANAR_MAX, face_frame};
use blackbox_gfx::{Fog, FrameParams, Instance, ReflectionQuality, ReflectionScene, RenderBackend};
use blackbox_scene::{Aabb, Frustum};
use glam::Vec3;

use super::{WorldScene, visibility};

/// Half the edge of the box a traffic car is tested in, metres: a car fits in a box this size around its origin.
const TRAFFIC_HALF_EXTENT: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CaptureStamp {
    seconds: f32,
    quality: ReflectionQuality,
    origin: Vec3,
    scenery: u64,
    original_shadows: bool,
    original_sky: bool,
    distance: ReflectionDistance,
    lod: ReflectionLod,
    content: ReflectionContent,
}

impl WorldScene {
    pub(super) fn update_reflections(&mut self, renderer: &mut dyn RenderBackend) {
        let _span = bevy_log::info_span!("world::reflections").entered();
        let graphics = *renderer.graphics();
        let quality = graphics.reflections;
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
        let (distance, lod, content) =
            (graphics.reflection_distance, graphics.reflection_lod, graphics.reflection_content);
        if self.reflection_last.is_some_and(|last| {
            last.quality == quality
                && last.distance == distance
                && last.lod == lod
                && last.content == content
                && last.scenery == scenery
                && last.original_shadows == self.original_shadows
                && last.original_sky == self.original_sky
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
            shadow_tuning: None,
        };
        // The cube face size the renderer captures at, so the detail threshold follows the capture.
        let output_height = renderer.surface_size()[1];
        let face_size = graphics.reflection_resolution.cube_face(quality.size(), output_height, PLANAR_MAX) as f32;
        // The traffic cars are gathered once; each face keeps the ones in its own frustum.
        let mut cars = Vec::new();
        if let Some(traffic) = self.traffic.as_ref().filter(|_| content.includes_traffic()) {
            traffic.instances(&mut cars);
        }
        let far = distance.car_far();
        for face in 0..6 {
            let frame = face_frame(origin, face, &template);
            let camera = visibility::Camera {
                light_dir: self.sun.direction().to_array(),
                view_proj: frame.view_proj(),
                position: origin.to_array(),
                forward: frame.view.inverse().transform_vector3(Vec3::NEG_Z).to_array(),
                fov_y_radians: std::f32::consts::FRAC_PI_2,
            };
            let placed = self
                .residency
                .placed()
                .filter(|p| self.original_shadows || !p.original_shadow)
                .filter(|p| within(origin, p.transform.w_axis.truncate(), far));
            let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
            // The traffic is appended after the scenery, so the face is sorted once after both (`collect_for_height`
            // has no unsorted variant: its own sort is the one extra pass on that path).
            match lod.uses_main_view() {
                true => visibility::collect_unsorted_with_shadows(
                    placed,
                    &camera,
                    rules,
                    self.residency.props(),
                    0.0,
                    &mut self.reflection_faces[face],
                ),
                false => visibility::collect_for_height(
                    placed,
                    &camera,
                    rules,
                    self.residency.props(),
                    face_size,
                    &mut self.reflection_faces[face],
                ),
            }
            add_traffic(
                &cars,
                origin,
                far,
                &Frustum::from_view_proj(&camera.view_proj),
                &mut self.reflection_faces[face],
            );
            if self.original_sky {
                self.residency.sky.place(
                    &self.residency.shared,
                    &mut self.reflection_faces[face],
                    origin,
                    super::sky::View::Environment,
                );
            }
            visibility::sort_by_mesh(&mut self.reflection_faces[face]);
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
            original_sky: self.original_sky,
            distance,
            lod,
            content,
        });
    }
}

/// Adds the traffic cars whose origin is within the far limit and whose box meets the face's frustum. The caller sorts
/// the face by mesh after this.
fn add_traffic(cars: &[Instance], origin: Vec3, far: Option<f32>, frustum: &Frustum, out: &mut Vec<Instance>) {
    let half = Vec3::splat(TRAFFIC_HALF_EXTENT);
    out.extend(cars.iter().copied().filter(|car| {
        let centre = car.transform.w_axis.truncate();
        within(origin, centre, far) && frustum.intersects(&Aabb { min: centre - half, max: centre + half })
    }));
}

#[cfg(test)]
#[path = "reflections_tests.rs"]
mod tests;
