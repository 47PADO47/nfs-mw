//! GPU checks of the view capture behind the rear-view mirror: it is polled to completion, a second one waits for
//! the first, and it leaves the main view's AO and surface images alone.

use std::time::Duration;

use glam::{Mat4, Vec3};

use super::test_support;
use blackbox_gfx::{
    AoQuality, AoSettings, CaptureId, FrameParams, GraphicsSettings, Projection, RenderBackend, RgbaImage,
};
use blackbox_gfx_testkit::camera::Camera;

const SIZE: (u32, u32) = (160, 90);

#[test]
#[ignore = "needs a GPU"]
fn hdr_and_srgb_outputs_capture_an_sdr_mirror_and_restore_the_main_output() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    let frame = clear_frame([2.0, 0.25, 0.05]);
    renderer.ui.set_hdr_white(Some(203.0));
    for format in [wgpu::TextureFormat::Rgba16Float, wgpu::TextureFormat::Bgra8UnormSrgb] {
        renderer.output = super::output::Output::texture_format(&renderer.device, (64, 36), format);
        for mapping in [None, Some(super::post::display::Mapping { peak: 5.0, white: 203.0, linear: true })] {
            renderer.post.set_display(&renderer.device, mapping);
            let expected = renderer.capture(16, 9, &frame, &[]).unwrap();
            let id = renderer.request_view_capture([16, 9], &frame, &[]).unwrap();
            assert_eq!(renderer.output.format(), format);
            assert_eq!(renderer.post.display, mapping);
            assert_eq!(renderer.ui.hdr_white(), Some(203.0));
            let image = poll_until_ready(&mut renderer, id).expect("HDR mirror picture completes");
            for y in 0..9 {
                for x in 0..16 {
                    let offset = ((y * 16 + x) * 4) as usize;
                    assert_eq!(&image.pixel(x, y).unwrap()[..], &expected[offset..offset + 4]);
                }
            }
        }
    }
}

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

/// Poll `id` until its picture is ready, at most 200 times. `None` if it never was.
fn poll_until_ready(backend: &mut dyn RenderBackend, id: CaptureId) -> Option<RgbaImage> {
    for _ in 0..200 {
        if let Some(result) = backend.poll_capture(id) {
            return Some(result.expect("the view capture reads back"));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    None
}

#[test]
#[ignore = "needs a GPU"]
fn a_view_capture_is_polled_until_its_picture_is_ready() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    let frame = clear_frame([1.0, 0.0, 0.5]);
    let backend: &mut dyn RenderBackend = &mut renderer;

    let id = backend.request_view_capture([16, 9], &frame, &[]).unwrap();
    let image = poll_until_ready(backend, id).expect("the picture is ready within 200 polls");
    assert_eq!((image.width, image.height), (16, 9));
    let pixel = image.pixel(8, 4).unwrap();
    assert_eq!([pixel[0], pixel[1], pixel[3]], [255, 0, 255]);
    // 0.5 is halfway between two UNORM bytes; both hardware rounding choices are valid.
    assert!((127..=128).contains(&pixel[2]));
    assert!(backend.poll_capture(id).is_none(), "each capture is returned once");
}

#[test]
#[ignore = "needs a GPU"]
fn hdr_effects_view_capture_is_sdr_and_restores_main_output() {
    use super::{output::Output, post::display::Mapping};

    let _gpu = test_support::serial();
    let mut renderer = test_support::headless((64, 36)).expect("HDR mirror adapter");
    renderer.output = Output::texture_format(&renderer.device, (64, 36), wgpu::TextureFormat::Rgba16Float);
    let mapping = Some(Mapping { peak: 1000.0 / 203.0, white: 203.0, linear: true });
    renderer.post.set_display(&renderer.device, mapping);
    renderer.ui.set_hdr_white(Some(203.0));
    renderer.refresh_targets();
    let frame = clear_frame([0.25, 0.5, 2.0]);
    let reference = renderer.capture(384, 131, &frame, &[]).unwrap();

    let id = renderer.request_view_capture([384, 131], &frame, &[]).unwrap();
    let image = poll_until_ready(&mut renderer, id).expect("the HDR mirror returns an SDR UI picture");
    assert_eq!(image.rgba.len(), 384 * 131 * 4);
    assert!(image.rgba.iter().zip(&reference).all(|(a, b)| a.abs_diff(*b) <= 1));
    assert_eq!(renderer.output.format(), wgpu::TextureFormat::Rgba16Float);
    assert_eq!(renderer.surface_size(), (64, 36));
    assert_eq!(renderer.post.display, mapping);
    assert_eq!(renderer.ui.hdr_white(), Some(203.0));
    assert!(renderer.render(&frame, &[]).unwrap(), "the main HDR frame still renders after the mirror");
}

#[test]
#[ignore = "needs a GPU"]
fn a_second_view_capture_waits_for_the_first_to_be_polled() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    let frame = clear_frame([0.2, 0.4, 0.6]);
    let backend: &mut dyn RenderBackend = &mut renderer;

    let first = backend.request_view_capture([16, 9], &frame, &[]).unwrap();
    assert!(backend.request_view_capture([16, 9], &frame, &[]).is_err(), "one view capture is in flight at a time");
    let image = poll_until_ready(backend, first).expect("the picture is ready within 200 polls");
    assert_eq!((image.width, image.height), (16, 9));
    assert!(backend.request_view_capture([8, 8], &frame, &[]).is_ok(), "the next one starts once the first is taken");
}

#[test]
#[ignore = "needs a GPU"]
fn a_view_capture_leaves_the_main_views_ao_and_surfaces_alone() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless(SIZE) else { return };
    let ao = AoSettings { quality: AoQuality::Low, intensity: 0.5, ..AoSettings::default() };
    renderer.apply_graphics(&GraphicsSettings { ao, ..GraphicsSettings::default() });
    let aspect = SIZE.0 as f32 / SIZE.1 as f32;
    let frame = Camera::new(Vec3::new(0.0, -6.0, 2.0), Vec3::new(0.0, 6.0, 2.0)).frame(aspect, [0.2, 0.3, 0.4], None);
    assert!(renderer.render(&frame, &[]).unwrap(), "a headless frame is always presented");
    let ao_before = renderer.ao.as_ref().map(|ao| ao.size);
    let surfaces_before = renderer.surfaces.as_ref().map(|s| s.targets.size);
    assert_eq!(ao_before, Some(SIZE), "the main frame built its AO image");
    assert!(surfaces_before.is_some(), "the main frame built its prepass surfaces");

    {
        let backend: &mut dyn RenderBackend = &mut renderer;
        let id = backend.request_view_capture([40, 22], &frame, &[]).unwrap();
        poll_until_ready(backend, id).expect("the picture is ready within 200 polls");
    }
    assert_eq!(renderer.ao.as_ref().map(|ao| ao.size), ao_before);
    assert_eq!(renderer.surfaces.as_ref().map(|s| s.targets.size), surfaces_before);
}
