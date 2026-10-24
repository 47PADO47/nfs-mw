//! The original local car environment capture. This is separate from wet-road reflections.
//!
//! One update draws only some of the cube's faces (`reflections_slices.rs`), so a refresh is spread over a few frames.
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

use super::reflections_visibility::{Listing, collect_tiles};
use super::shadow_feed::cars::User as CarsUser;
use super::{WorldScene, visibility};

#[path = "reflections_cadence.rs"]
mod cadence;
#[path = "reflections_slices.rs"]
mod slices;

/// Half the edge of the box a traffic car is tested in, metres: a car fits in a box this size around its origin.
const TRAFFIC_HALF_EXTENT: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CaptureStamp {
    /// Where and when the refresh under way began, and the faces it has left.
    progress: slices::Progress,
    cached: bool,
    quality: ReflectionQuality,
    size: u32,
    scenery: u64,
    original_shadows: bool,
    original_sky: bool,
    enhanced_sky: bool,
    sky_tuning: [f32; 5],
    distance: ReflectionDistance,
    lod: ReflectionLod,
    content: ReflectionContent,
    custom: bool,
}

impl WorldScene {
    pub(super) fn update_reflections(&mut self, renderer: &mut dyn RenderBackend) {
        let _trace = blackbox_trace::scope("World: reflection views");
        let _span = bevy_log::info_span!("world::reflections").entered();
        let graphics = *renderer.graphics();
        let quality = graphics.reflections;
        let Some(drive) = self.players.lead() else {
            renderer.set_reflection_scene(None);
            self.reflection_last = None;
            return;
        };
        if quality == ReflectionQuality::Off || self.car_shading == crate::settings::CarShading::Simple {
            renderer.set_reflection_scene(None);
            self.reflection_last = None;
            return;
        }
        // Cadence, slicing and resolution are rewrite choices; the faces of one update share one snapshot. Never
        // accumulate temporal history: at high speed the previous world must not trail across the paint.
        let cached = renderer.render_experiments().cached_reflections;
        let options = renderer.render_experiments();
        let sky_tuning = [
            options.sky_wind_speed,
            options.sky_sun_size,
            options.sky_sun_brightness,
            options.sky_brightness,
            options.sky_sunset_warmth,
        ];
        let origin = drive.pose().position + Vec3::Z * 0.95;
        let size = graphics.reflection_resolution.cube_face(quality.size(), renderer.surface_size()[1], PLANAR_MAX);
        let scenery = self.residency.visual_revision();
        let (distance, lod, content) =
            (graphics.reflection_distance, graphics.reflection_lod, graphics.reflection_content);
        let previous = self.reflection_last;
        let same_cube = |last: &CaptureStamp| last.cached == cached && last.quality == quality && last.size == size;
        let content_changed = previous.is_some_and(|last| {
            last.distance != distance
                || last.lod != lod
                || last.content != content
                || last.custom == self.car_shading.is_original()
                || last.scenery != scenery
                || last.original_shadows != self.original_shadows
                || last.original_sky != self.original_sky
                || last.enhanced_sky != options.enhanced_sky
                || last.sky_tuning != sky_tuning
        });
        let inputs = slices::Inputs {
            cached,
            quality,
            now: self.clock,
            origin,
            same_cube: previous.as_ref().is_some_and(same_cube),
            content_changed,
        };
        let Some((progress, faces)) = slices::plan(previous.map(|last| last.progress), &inputs) else { return };
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
        let face_size = size as f32;
        let far = distance.car_far();
        let cameras = std::array::from_fn(|face| {
            let frame = face_frame(origin, face, &template);
            visibility::Camera {
                light_dir: self.sun.direction().to_array(),
                view_proj: frame.view_proj(),
                position: origin.to_array(),
                forward: frame.view.inverse().transform_vector3(Vec3::NEG_Z).to_array(),
                fov_y_radians: std::f32::consts::FRAC_PI_2,
            }
        });
        let listing = Listing {
            original_shadows: self.original_shadows,
            // The Original PC capture keeps its documented origin-distance selection.
            origin_far: match self.car_shading.is_original() {
                true => far,
                false => None,
            },
        };
        let height = match lod.uses_main_view() {
            true => visibility::REFERENCE_HEIGHT,
            false => face_size,
        };
        collect_tiles(
            self.residency.cull_tiles(),
            listing,
            &cameras,
            &blackbox_scenery::layout::MOST_WANTED.lod,
            self.residency.props(),
            height,
            match self.car_shading.is_original() {
                true => None,
                false => far,
            },
            faces,
            &mut self.reflection_faces,
            &mut self.reflection_scratch,
        );
        // The traffic cars are gathered once for the frame (shared with the shadow feed); each face keeps the ones in
        // its own frustum.
        let cars = match self.traffic.as_ref().filter(|_| content.includes_traffic() && faces != 0) {
            Some(traffic) => self.shadow_feed.cars.get(traffic, CarsUser::Reflections),
            None => &[],
        };
        // The faces left out keep their last lists, which the renderer does not read.
        for (face, camera) in cameras.iter().enumerate().filter(|(face, _)| (faces >> face) & 1 != 0) {
            add_traffic(
                cars,
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
            sun_shadows: !self.car_shading.is_original(),
            fade_distance: match self.car_shading.is_original() {
                true => None,
                false => far,
            },
            origin,
            quality,
            frame: template,
            faces: self.reflection_faces.each_ref().map(Vec::as_slice),
            refresh: faces,
        }));
        self.reflection_last = Some(CaptureStamp {
            progress,
            cached,
            quality,
            size,
            scenery,
            original_shadows: self.original_shadows,
            original_sky: self.original_sky,
            enhanced_sky: options.enhanced_sky,
            sky_tuning,
            distance,
            lod,
            content,
            custom: !self.car_shading.is_original(),
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
