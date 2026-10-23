//! GPU checks of the pipeline prewarm on a headless renderer.

use super::test_support;
use crate::FrameParams;
use blackbox_gfx::{GraphicsSettings, Projection, ReflectionQuality, RenderBackend};
use glam::{Mat4, Vec3};

fn frame() -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 16.0 / 9.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.2, 0.4, 0.6],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}

#[test]
#[ignore = "needs a GPU"]
fn prewarm_builds_the_glossy_state_once_and_frames_still_draw() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    assert!(!renderer.glossy_in_use());
    assert!(renderer.prewarmed.is_none());

    renderer.prewarm();
    assert!(renderer.glossy_in_use());
    assert!(renderer.pipelines.has_glossy());
    assert!(renderer.prewarmed.is_some());

    // Nothing changed, so the second call has nothing to do and leaves the same state behind.
    let key = renderer.warm_key();
    renderer.prewarm();
    assert!(renderer.prewarmed.as_ref() == Some(&key));

    assert!(renderer.render(&frame(), &[]).unwrap(), "a headless frame is always presented");
    assert_eq!(renderer.read_output().unwrap().unwrap().pixel(0, 0), Some([51, 102, 153, 255]));
}

#[test]
#[ignore = "needs a GPU"]
fn a_settings_change_is_prewarmed_on_request_only_when_headless() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    let wanted = GraphicsSettings { reflections: ReflectionQuality::Medium, ..GraphicsSettings::default() };
    RenderBackend::apply_graphics(&mut renderer, &wanted);
    assert!(renderer.prewarmed.is_none(), "a headless renderer does not draw behind the caller's back");

    renderer.prewarm();
    let before = renderer.warm_key();
    assert!(renderer.prewarmed.as_ref() == Some(&before));
    let wanted_reflections = renderer.graphics().reflections != ReflectionQuality::Off;
    assert_eq!(renderer.reflections.is_some(), wanted_reflections, "the cube capture is built when reflections are on");

    // A changed setting makes the next call do the work again.
    RenderBackend::apply_graphics(&mut renderer, &GraphicsSettings::default());
    assert!(renderer.prewarmed.as_ref() != Some(&renderer.warm_key()));
    renderer.prewarm();
    assert!(renderer.prewarmed.as_ref() == Some(&renderer.warm_key()));
}
