//! GPU checks of the headless renderer and of capturing through the `RenderBackend` trait.

use glam::{Mat4, Vec3};

use super::test_support;
use crate::{FrameParams, Renderer, RendererOptions};
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
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
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
    let pixel = image.pixel(8, 4).unwrap();
    assert_eq!([pixel[0], pixel[1], pixel[3]], [255, 0, 255]);
    // 0.5 is halfway between two UNORM bytes; both hardware rounding choices are valid.
    assert!((127..=128).contains(&pixel[2]));
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

#[test]
#[ignore = "needs a GPU"]
fn the_vulkan_pipeline_cache_is_saved_and_read_back() {
    let _gpu = test_support::serial();
    let dir = std::env::temp_dir().join(format!("blackbox-pipeline-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let with_dir =
        |dir: &std::path::Path| RendererOptions { pipeline_cache_dir: Some(dir.to_owned()), ..test_support::options() };
    let mut renderer = match Renderer::headless((64, 36), with_dir(dir.as_path())) {
        Ok(renderer) => renderer,
        Err(e) => return eprintln!("skipping: {e}"),
    };
    renderer.render(&clear_frame([0.2, 0.4, 0.6]), &[]).unwrap();
    if renderer.adapter_info.backend != wgpu::Backend::Vulkan || renderer.pipeline_cache.is_none() {
        return eprintln!("skipping: this adapter has no Vulkan pipeline cache");
    }

    renderer.save_pipeline_cache();
    let path = renderer.pipeline_cache_path.clone().expect("a cache has a file");
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    assert!(size > 0, "the pipeline cache is saved to {}", path.display());
    drop(renderer);

    // A second renderer reads the saved file back without error.
    let second = Renderer::headless((64, 36), with_dir(dir.as_path())).expect("a second renderer on the same adapter");
    assert!(second.pipeline_cache.is_some());
    drop(second);
    let _ = std::fs::remove_dir_all(&dir);
}
