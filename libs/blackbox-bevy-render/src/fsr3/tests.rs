//! The pure parts of FSR 3 on the Bevy backend: the sizes it draws at, the jitter sequence for each quality mode, the
//! motion-vector scale, the projection read-back, the caps offer and the component plan. No GPU.

use bevy_camera::{CameraProjection, Hdr, OrthographicProjection, PerspectiveProjection};
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy_ecs::system::{Commands, RunSystemOnce};
use bevy_ecs::world::World;
use bevy_render::camera::{MipBias, TemporalJitter};
use blackbox_gfx::{
    Antialiasing, GraphicsApi, GraphicsSettings, PostSettings, RtSupport, UpscaleQuality, Upscaler, resolve,
    suggested_temporal_texture_lod_bias,
};

use super::component::{Fsr3Upscale, set};
use super::sizes;
use crate::caps::capabilities_with_ray_tracing;
use crate::ops::CameraSettings;

const DISPLAY: [u32; 2] = [1920, 1080];

/// Every quality mode the menus offer, with the render size it draws at on a 1920x1080 display.
const QUALITY_SIZES: [(UpscaleQuality, [u32; 2]); 5] = [
    (UpscaleQuality::Native, [1920, 1080]),
    (UpscaleQuality::Quality, [1280, 720]),
    (UpscaleQuality::Balanced, [1129, 635]),
    (UpscaleQuality::Performance, [960, 540]),
    (UpscaleQuality::UltraPerformance, [640, 360]),
];

#[test]
fn quality_modes_render_at_the_shared_table_size() {
    for (quality, want) in QUALITY_SIZES {
        assert_eq!(sizes::render_size(DISPLAY, quality.render_scale()), want, "{quality:?}");
    }
}

#[test]
fn a_render_scale_of_one_draws_at_the_display_size_and_nothing_is_upscaled() {
    assert_eq!(sizes::render_size(DISPLAY, 1.0), DISPLAY);
    assert_eq!(sizes::render_size(DISPLAY, 1.5), DISPLAY);
}

#[test]
fn the_jitter_cycle_is_the_phase_count_for_each_quality_mode() {
    // AMD's counts: 8 at 1x, 8 * ratio^2 otherwise (18 at 1.5x, 32 at 2x, 72 at 3x).
    let phases = [8, 18, 23, 32, 72];
    for ((quality, _), want) in QUALITY_SIZES.into_iter().zip(phases) {
        let render = sizes::render_size(DISPLAY, quality.render_scale());
        let count = fsr3_wgpu::jitter::phase_count(render[0], DISPLAY[0]);
        assert_eq!(count, want, "{quality:?}");
        // The offsets repeat with that period, stay inside the pixel and are all different within one cycle.
        let offsets: Vec<[f32; 2]> = (0..count).map(|i| sizes::jitter(i, render, DISPLAY)).collect();
        for (i, offset) in offsets.iter().enumerate() {
            assert!(offset.iter().all(|v| (-0.5..0.5).contains(v)), "{quality:?} {offset:?}");
            assert_eq!(*offset, sizes::jitter(i as u32 + count, render, DISPLAY), "{quality:?}");
            assert!(offsets[i + 1..].iter().all(|other| other != offset), "{quality:?} repeats within a cycle");
        }
    }
}

#[test]
fn the_jitter_is_zero_mean_over_a_cycle() {
    // A Halton sequence covers the pixel evenly, so the jitter averages to (almost) nothing over a full cycle.
    let render = sizes::render_size(DISPLAY, UpscaleQuality::Quality.render_scale());
    let count = fsr3_wgpu::jitter::phase_count(render[0], DISPLAY[0]);
    let (sum_x, sum_y) = (0..count).fold((0.0, 0.0), |(x, y), i| {
        let [dx, dy] = sizes::jitter(i, render, DISPLAY);
        (x + dx, y + dy)
    });
    assert!((sum_x / count as f32).abs() < 0.05 && (sum_y / count as f32).abs() < 0.05, "{sum_x} {sum_y}");
}

#[test]
fn the_motion_vector_scale_turns_bevy_uv_motion_into_previous_minus_current_pixels() {
    let render = [1280, 720];
    let scale = sizes::motion_vector_scale(render);
    assert_eq!(scale, [-1280.0, -720.0]);
    // A point that moved 10 pixels to the right: Bevy's motion is current minus previous in UV, so +10 / width.
    let bevy_uv_motion_x = 10.0 / 1280.0;
    assert!((scale[0] * bevy_uv_motion_x - -10.0).abs() < 1e-4);
}

#[test]
fn the_sharpness_setting_maps_to_the_upscaler_or_to_no_pass() {
    assert_eq!(sizes::sharpness(0.0), None);
    assert_eq!(sizes::sharpness(0.8), Some(0.8));
    assert_eq!(sizes::sharpness(3.0), Some(1.0));
    assert_eq!(sizes::sharpness(f32::NAN), Some(0.8));
}

#[test]
fn the_near_plane_and_fov_are_read_back_from_bevys_projection() {
    let (fov, aspect, near) = (1.0_f32, 16.0 / 9.0, 0.25_f32);
    let lens = PerspectiveProjection { fov, aspect_ratio: aspect, near, far: 1.0e6, ..Default::default() };
    let mut projection = lens.get_clip_from_view();
    let (got_near, got_fov) = sizes::near_and_fov_y(projection).expect("a perspective projection");
    assert!((got_near - near).abs() < 1e-6 && (got_fov - fov).abs() < 1e-5, "{got_near} {got_fov}");
    // The jitter only moves the translation column of the z axis, so it does not change either value.
    projection.z_axis.x += 0.3;
    projection.z_axis.y -= 0.2;
    let (jittered_near, jittered_fov) = sizes::near_and_fov_y(projection).expect("still perspective");
    assert!((jittered_near - near).abs() < 1e-6 && (jittered_fov - fov).abs() < 1e-5);
}

#[test]
fn an_orthographic_projection_has_no_near_plane_for_the_upscaler() {
    let ortho = OrthographicProjection::default_3d().get_clip_from_view();
    assert_eq!(sizes::near_and_fov_y(ortho), None);
}

#[test]
fn the_caps_offer_fsr3_only_when_the_device_can_run_it() {
    let offered = capabilities_with_ray_tracing(GraphicsApi::Vulkan, true, false, RtSupport::None, true);
    assert!(offered.upscalers.contains(Upscaler::Fsr3));
    let refused = capabilities_with_ray_tracing(GraphicsApi::Vulkan, true, false, RtSupport::None, false);
    assert!(!refused.upscalers.contains(Upscaler::Fsr3));
}

#[test]
fn a_fsr3_request_resolves_to_fsr3_at_the_quality_scale_with_taa_off() {
    let caps = capabilities_with_ray_tracing(GraphicsApi::Vulkan, true, false, RtSupport::None, true);
    for (quality, _) in QUALITY_SIZES {
        let request = GraphicsSettings {
            upscaler: Upscaler::Fsr3,
            upscale_quality: quality,
            post: PostSettings { antialiasing: Antialiasing::Taa, ..PostSettings::default() },
            ..GraphicsSettings::default()
        };
        let effective = resolve(&request, &caps).effective;
        assert_eq!(effective.upscaler, Upscaler::Fsr3, "{quality:?}");
        assert_eq!(effective.render_scale, quality.render_scale(), "{quality:?}");
        // A temporal upscaler does its own anti-aliasing, so TAA is turned off, not doubled.
        assert_eq!(effective.post.antialiasing, Antialiasing::Off, "{quality:?}");
    }
}

#[test]
fn a_fsr3_request_falls_back_to_fsr1_when_the_device_cannot_run_it() {
    let caps = capabilities_with_ray_tracing(GraphicsApi::Vulkan, true, false, RtSupport::None, false);
    let request = GraphicsSettings { upscaler: Upscaler::Fsr3, ..GraphicsSettings::default() };
    assert_eq!(resolve(&request, &caps).effective.upscaler, Upscaler::Fsr1);
}

/// The settings as `apply_graphics` would leave them for a FSR 3 request at `quality`.
fn fsr3_settings(quality: UpscaleQuality) -> CameraSettings {
    CameraSettings {
        upscaler: Upscaler::Fsr3,
        render_scale: quality.render_scale(),
        mip_bias: suggested_temporal_texture_lod_bias(quality.render_scale()),
        ..CameraSettings::default()
    }
}

fn put(world: &mut World, camera: bevy_ecs::entity::Entity, settings: CameraSettings, camera_cut: bool) {
    world
        .run_system_once(move |mut commands: Commands| set(&mut commands, camera, &settings, camera_cut))
        .expect("set");
}

#[test]
fn the_camera_gets_the_fsr3_components_when_fsr3_is_the_upscaler() {
    let mut world = World::new();
    let camera = world.spawn_empty().id();
    let settings = fsr3_settings(UpscaleQuality::Quality);
    put(&mut world, camera, settings, true);

    assert_eq!(world.get::<Fsr3Upscale>(camera), Some(&Fsr3Upscale { reset: true }));
    assert!(world.get::<DepthPrepass>(camera).is_some(), "the depth prepass");
    assert!(world.get::<MotionVectorPrepass>(camera).is_some(), "the motion vectors");
    assert!(world.get::<TemporalJitter>(camera).is_some(), "the jitter");
    assert!(world.get::<Hdr>(camera).is_some(), "linear HDR colour");
    let bias = world.get::<MipBias>(camera).expect("the temporal mip bias").0;
    assert!((bias - suggested_temporal_texture_lod_bias(UpscaleQuality::Quality.render_scale())).abs() < 1e-6);
}

#[test]
fn the_camera_loses_the_fsr3_marker_when_fsr3_is_not_the_upscaler() {
    let mut world = World::new();
    let camera = world.spawn_empty().id();
    put(&mut world, camera, fsr3_settings(UpscaleQuality::Quality), false);
    assert!(world.get::<Fsr3Upscale>(camera).is_some());

    let fsr1 = CameraSettings { upscaler: Upscaler::Fsr1, render_scale: 0.67, ..CameraSettings::default() };
    put(&mut world, camera, fsr1, false);
    assert!(world.get::<Fsr3Upscale>(camera).is_none(), "the marker goes when FSR 3 is off");

    let fresh = world.spawn_empty().id();
    put(&mut world, fresh, CameraSettings { upscaler: Upscaler::Off, ..CameraSettings::default() }, false);
    assert!(world.get::<Fsr3Upscale>(fresh).is_none());
    assert!(world.get::<TemporalJitter>(fresh).is_none(), "a camera that never had FSR 3 gets no jitter");
}

#[test]
fn a_camera_cut_is_carried_to_the_render_world_as_the_reset_flag() {
    let mut world = World::new();
    let camera = world.spawn_empty().id();
    put(&mut world, camera, fsr3_settings(UpscaleQuality::Balanced), false);
    assert_eq!(world.get::<Fsr3Upscale>(camera), Some(&Fsr3Upscale { reset: false }));
    put(&mut world, camera, fsr3_settings(UpscaleQuality::Balanced), true);
    assert_eq!(world.get::<Fsr3Upscale>(camera), Some(&Fsr3Upscale { reset: true }));
}
