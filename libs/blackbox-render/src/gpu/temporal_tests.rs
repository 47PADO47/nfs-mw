//! GPU checks of the native renderer's FSR 3 path on a real device: what it offers, the motion vectors of the
//! camera and of a moving object, that a static scene settles, and that a capture does not touch the history.
//! Run with `cargo test -p blackbox-render --lib temporal -- --ignored`.

use blackbox_gfx::{
    BlendMode, FrameParams, GraphicsSettings, Instance, InstanceKey, RenderBackend, RgbaImage, Shading, UpscaleQuality,
    Upscaler, scaled_size, suggested_temporal_texture_lod_bias,
};
use blackbox_gfx_testkit::camera::Camera;
use blackbox_gfx_testkit::mesh::{MeshBuilder, NEUTRAL};
use blackbox_gfx_testkit::{Mask, SceneId, compare};
use glam::{Mat4, Vec3};

use super::test_support;
use crate::Renderer;

const SIZE: (u32, u32) = (160, 90);

fn fsr3(quality: UpscaleQuality) -> GraphicsSettings {
    GraphicsSettings { upscaler: Upscaler::Fsr3, upscale_quality: quality, ..GraphicsSettings::default() }
}

/// A headless renderer that offers FSR 3, or `None` (with a note) when this machine has no such device.
fn renderer() -> Option<(std::sync::MutexGuard<'static, ()>, Renderer)> {
    let guard = test_support::serial();
    let renderer = test_support::headless(SIZE)?;
    if !renderer.capabilities().upscalers.contains(Upscaler::Fsr3) {
        eprintln!("skipping: this device does not offer FSR 3");
        return None;
    }
    Some((guard, renderer))
}

#[test]
#[ignore = "needs a GPU"]
fn fsr3_is_offered_and_leads_the_chain_while_it_is_chosen() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    assert!(renderer.post_passes().is_empty());
    assert!(renderer.draws_directly(), "the default build draws straight into the output");

    let resolved = renderer.apply_graphics(&fsr3(UpscaleQuality::Quality));
    assert!(resolved.is_exact(), "{:?}", resolved.downgrades);
    let scale = UpscaleQuality::Quality.render_scale();
    assert_eq!(renderer.post_passes(), ["fsr3"]);
    assert!(!renderer.draws_directly() && renderer.is_hdr(), "FSR 3 takes the scene as HDR");
    assert_eq!(renderer.render_size(), scaled_size(SIZE, scale));
    assert_eq!(renderer.texture_lod_bias(), suggested_temporal_texture_lod_bias(scale));
    assert!(!renderer.fsr1_active());

    renderer.apply_graphics(&GraphicsSettings { upscaler: Upscaler::Fsr1, render_scale: 0.5, ..Default::default() });
    assert!(renderer.fsr1_active() && !renderer.post_passes().contains(&"fsr3"));
    renderer.apply_graphics(&GraphicsSettings::default());
    assert!(renderer.post_passes().is_empty());
    assert!(renderer.draws_directly(), "choosing something else puts the default path back");
}

/// A wall 16 m ahead and a small card 10 m ahead, both keyed.
struct Walls {
    wall: blackbox_gfx::MeshHandle,
    card: blackbox_gfx::MeshHandle,
}

impl Walls {
    fn new(backend: &mut dyn RenderBackend) -> Self {
        let mut builder = MeshBuilder::new();
        builder.state(None, BlendMode::Opaque, Shading::Prelit).card(
            Vec3::new(0.0, 10.0, -30.0),
            200.0,
            200.0,
            0.0,
            NEUTRAL,
        );
        let wall = builder.finish().upload(backend, "motion wall");
        let mut builder = MeshBuilder::new();
        builder.state(None, BlendMode::Opaque, Shading::Prelit).card(Vec3::new(0.0, 4.0, 0.0), 2.0, 4.0, 0.0, NEUTRAL);
        let card = builder.finish().upload(backend, "motion card");
        Self { wall, card }
    }

    fn instances(&self, card_x: f32) -> [Instance; 2] {
        [
            Instance::keyed(self.wall, Mat4::IDENTITY, InstanceKey::new(1, 0)),
            Instance::keyed(self.card, Mat4::from_translation(Vec3::X * card_x), InstanceKey::new(1, 1)),
        ]
    }
}

fn frame(eye_x: f32, cut: bool) -> FrameParams {
    let aspect = SIZE.0 as f32 / SIZE.1 as f32;
    let mut frame =
        Camera::new(Vec3::new(eye_x, -6.0, 2.0), Vec3::new(eye_x, 6.0, 2.0)).frame(aspect, [0.2, 0.3, 0.4], None);
    frame.camera_cut = cut;
    frame
}

/// Read the Rg16Float motion image back as (x, y) pairs, row by row.
fn read_motion(renderer: &Renderer) -> ((u32, u32), Vec<[f32; 2]>) {
    let scene = renderer.targets.offscreen().expect("an offscreen scene");
    let texture = scene.motion.as_ref().expect("a motion image").texture();
    let (width, height) = scene.size;
    let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("motion readback"),
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    renderer.queue.submit([encoder.finish()]);
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    renderer.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
    let mapped = readback.slice(..).get_mapped_range().expect("mapped");
    let mut pixels = Vec::new();
    for y in 0..height as usize {
        let line = &mapped[y * row as usize..][..width as usize * 4];
        for texel in line.as_chunks::<4>().0 {
            let half = |lo: usize| half_to_f32(u16::from_le_bytes([texel[lo], texel[lo + 1]]));
            pixels.push([half(0), half(2)]);
        }
    }
    ((width, height), pixels)
}

/// An IEEE half as a float (no NaN or infinity handling; vectors are small).
fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * fraction * 2f32.powi(-24),
        _ => sign * (1.0 + fraction / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// The normalised device x of a world point 10 or 16 m ahead is `x * focal / distance`; the camera at x moves the
/// whole view the other way.
fn focal_x() -> f32 {
    let aspect = SIZE.0 as f32 / SIZE.1 as f32;
    1.0 / (30f32.to_radians().tan() * aspect)
}

fn pixel_of(ndc: f32, size: u32) -> usize {
    ((ndc * 0.5 + 0.5) * size as f32) as usize
}

#[test]
#[ignore = "needs a GPU"]
fn the_motion_vectors_hold_the_camera_motion_and_the_motion_of_a_keyed_object() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    renderer.apply_graphics(&fsr3(UpscaleQuality::Native));
    let walls = Walls::new(&mut renderer);

    // Frame one is a cut. In frame two the camera has moved 0.2 m right and the card 0.5 m right.
    renderer.render(&frame(0.0, true), &walls.instances(0.0)).expect("render");
    renderer.render(&frame(0.2, false), &walls.instances(0.5)).expect("render");
    let ((width, height), motion) = read_motion(&renderer);
    let at = |x: usize, y: usize| motion[y * width as usize + x];

    // The wall is 16 m away along the view axis: the camera moving right moves it left on screen, so the
    // previous position minus the current one is to the right.
    let wall = at(width as usize / 8, height as usize / 2);
    let want = 0.2 * focal_x() / 16.0;
    assert!((wall[0] - want).abs() < 2e-4 && wall[1].abs() < 2e-4, "wall: got {wall:?}, want ({want}, 0)");

    // The card is 10 m away. Relative to the camera it moved 0.3 m right, so it was 0.3 m to the left.
    let card_centre = pixel_of((0.5 - 0.2) * focal_x() / 10.0, width);
    let card = at(card_centre, height as usize / 2);
    let want = -0.3 * focal_x() / 10.0;
    assert!((card[0] - want).abs() < 2e-4 && card[1].abs() < 2e-4, "card: got {card:?}, want ({want}, 0)");
}

#[test]
#[ignore = "needs a GPU"]
fn a_camera_cut_and_an_unkeyed_instance_have_no_motion_of_their_own() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    renderer.apply_graphics(&fsr3(UpscaleQuality::Native));
    let walls = Walls::new(&mut renderer);
    let at = |x: f32| Mat4::from_translation(Vec3::X * x);
    let biggest = |renderer: &Renderer| {
        let (_, motion) = read_motion(renderer);
        motion.iter().flatten().fold(0.0f32, |most, v| most.max(v.abs()))
    };

    // A static camera and an unkeyed card that moves 0.5 m: nothing says where it was, so it has no vector.
    let wall = walls.instances(0.0)[0];
    let unkeyed = |x| [wall, Instance::new(walls.card, at(x))];
    renderer.render(&frame(0.0, true), &unkeyed(0.0)).expect("render");
    renderer.render(&frame(0.0, false), &unkeyed(0.5)).expect("render");
    let most = biggest(&renderer);
    assert!(most < 1e-4, "an unkeyed instance has the camera's motion only, but a vector of {most} is there");

    // A keyed card that moves 0.5 m has a vector, unless the frame is a camera cut.
    renderer.render(&frame(0.0, false), &walls.instances(0.0)).expect("render");
    renderer.render(&frame(0.0, false), &walls.instances(0.5)).expect("render");
    assert!(biggest(&renderer) > 1e-2, "the keyed card moved");
    renderer.render(&frame(0.0, true), &walls.instances(0.0)).expect("render");
    assert!(biggest(&renderer) < 1e-4, "a cut trusts no history");
}

fn render_frames(renderer: &mut Renderer, scene: &blackbox_gfx_testkit::BuiltScene, frames: usize) -> RgbaImage {
    for _ in 0..frames {
        renderer.render(&scene.frame, &scene.instances).expect("render");
    }
    renderer.read_output().expect("a headless output").expect("readback")
}

#[test]
#[ignore = "needs a GPU"]
fn fsr3_converges_on_a_static_scene() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    renderer.apply_graphics(&fsr3(UpscaleQuality::Quality));
    let built = SceneId::Grid.build(&mut renderer, [SIZE.0, SIZE.1]);
    built.apply_layers(&mut renderer);

    // Settle past the jitter cycle (18 frames at 1.5x) before the first sample.
    let settled = render_frames(&mut renderer, &built, 24);
    let later = render_frames(&mut renderer, &built, 16);
    built.release(&mut renderer);

    let diff = compare(&settled, &later, &Mask::all()).expect("same size");
    eprintln!("native fsr3 convergence, frame N vs N+16: {diff}");
    assert!(diff.mean < 2.0 && diff.p99 <= 16, "a settled FSR 3 picture should barely change 16 frames later: {diff}");
}

/// FSR 3 on a camera that slides sideways, against the picture the same camera gives at the output size with no
/// upscaler. Prints the difference at Quality and at the native ratio, for FSR 3 and for bilinear at the same
/// render size; it is a measurement, not a gate. Needs `--nocapture` to see the numbers.
#[test]
#[ignore = "needs a GPU; prints the moving-camera numbers"]
fn fsr3_on_a_moving_camera_tracks_the_native_picture() {
    let _gpu = test_support::serial();
    let frames = 30;
    let slide = |settings: &GraphicsSettings| -> Option<RgbaImage> {
        let mut renderer = test_support::headless(SIZE)?;
        if settings.upscaler == Upscaler::Fsr3 && !renderer.capabilities().upscalers.contains(Upscaler::Fsr3) {
            return None;
        }
        renderer.apply_graphics(settings);
        let built = SceneId::Grid.build(&mut renderer, [SIZE.0, SIZE.1]);
        built.apply_layers(&mut renderer);
        let aspect = SIZE.0 as f32 / SIZE.1 as f32;
        let fog = built.frame.fog.as_ref().map(|f| (f.start, f.end));
        for k in 0..frames {
            let x = 0.05 * k as f32;
            let frame = Camera::new(Vec3::new(x, -6.0, 4.5), Vec3::new(x, 14.0, 0.0)).frame(
                aspect,
                built.frame.clear_color,
                fog,
            );
            renderer.render(&frame, &built.instances).expect("render");
        }
        let image = renderer.read_output()?.ok();
        built.release(&mut renderer);
        image
    };
    let Some(native) = slide(&GraphicsSettings::default()) else { return };
    for (label, quality) in [("quality", UpscaleQuality::Quality), ("native ratio", UpscaleQuality::Native)] {
        let Some(upscaled) = slide(&fsr3(quality)) else { return };
        let bilinear = slide(&GraphicsSettings {
            upscaler: Upscaler::Bilinear,
            render_scale: quality.render_scale(),
            ..GraphicsSettings::default()
        });
        let diff = compare(&native, &upscaled, &Mask::all()).expect("same size");
        eprintln!("native fsr3 moving camera, {label} vs native: {diff}");
        let Some(bilinear) = bilinear else { continue };
        let diff = compare(&native, &bilinear, &Mask::all()).expect("same size");
        eprintln!("bilinear at the same render size, {label} vs native: {diff}");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn a_capture_under_fsr3_is_drawn_at_the_output_size_without_the_upscaler() {
    let Some((_gpu, mut renderer)) = renderer() else { return };
    let built = SceneId::Grid.build(&mut renderer, [SIZE.0, SIZE.1]);
    built.apply_layers(&mut renderer);
    let plain = renderer.capture(SIZE.0, SIZE.1, &built.frame, &built.instances).expect("capture");

    renderer.apply_graphics(&fsr3(UpscaleQuality::Performance));
    assert_eq!(renderer.post_passes()[0], "fsr3");
    renderer.render(&built.frame, &built.instances).expect("render");
    let captured = renderer.capture(SIZE.0, SIZE.1, &built.frame, &built.instances).expect("capture");
    built.release(&mut renderer);

    let image = |rgba| RgbaImage::new(SIZE.0, SIZE.1, rgba).expect("image");
    let diff = compare(&image(plain), &image(captured), &Mask::all()).expect("same size");
    assert!(diff.mean < 0.5, "a capture has no history to accumulate, so it matches the plain picture: {diff}");
}
