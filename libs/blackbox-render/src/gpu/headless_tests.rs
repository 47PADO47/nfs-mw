//! GPU checks of the headless renderer and of capturing through the `RenderBackend` trait.

use glam::{Mat4, Vec3};

use super::test_support;
use crate::{FrameParams, Renderer};
use blackbox_gfx::Projection;
use blackbox_gfx::{FrameStatus, RenderBackend};

fn clear_frame(color: [f32; 3]) -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: color,
        fog: None,
        camera_cut: false,
    }
}

#[test]
#[ignore = "needs a GPU"]
fn a_headless_renderer_draws_into_its_output() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    assert_eq!(renderer.surface_size(), (64, 36));
    assert!(renderer.draws_directly());
    assert!(renderer.read_output().is_some());

    let frame = clear_frame([0.2, 0.4, 0.6]);
    assert!(renderer.render(&frame, &[]).unwrap(), "a headless frame is always presented");
    let image = renderer.read_output().unwrap().unwrap();
    assert_eq!((image.width, image.height), (64, 36));
    for (x, y) in [(0, 0), (63, 35), (30, 17)] {
        assert_eq!(image.pixel(x, y), Some([51, 102, 153, 255]), "({x}, {y})");
    }

    renderer.resize(32, 18);
    assert_eq!(renderer.surface_size(), (32, 18));
    let image = renderer.read_output().unwrap().unwrap();
    assert_eq!((image.width, image.height), (32, 18));
}

#[test]
#[ignore = "needs a GPU"]
fn captures_are_requested_then_polled_once() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    let frame = clear_frame([1.0, 0.0, 0.5]);
    let backend: &mut dyn RenderBackend = &mut renderer;

    let first = backend.request_capture([48, 27], &frame, &[]).unwrap();
    let second = backend.request_capture([16, 9], &frame, &[]).unwrap();
    assert_ne!(first, second);

    let image = backend.poll_capture(second).expect("the native backend completes at once").unwrap();
    assert_eq!((image.width, image.height), (16, 9));
    assert_eq!(image.pixel(8, 4), Some([255, 0, 128, 255]));
    assert!(backend.poll_capture(second).is_none(), "each capture is returned once");

    let image = backend.poll_capture(first).unwrap().unwrap();
    assert_eq!((image.width, image.height), (48, 27));
    assert!(backend.request_capture([0, 9], &frame, &[]).is_err());
}

#[test]
#[ignore = "needs a GPU"]
fn the_trait_reports_the_adapter_and_status() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 18)) else { return };
    let backend: &mut dyn RenderBackend = &mut renderer;
    let info = backend.info().clone();
    assert_eq!(info.renderer, "blackbox");
    assert!(!info.adapter.is_empty());
    assert_eq!(backend.capabilities().api, info.api);
    assert_eq!(backend.aspect_ratio(), 32.0 / 18.0);
    let status = backend.render(&clear_frame([0.0; 3]), &[]).unwrap();
    assert_eq!(status, FrameStatus::Presented);
    let _: &Renderer = &renderer;
}
