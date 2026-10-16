//! FSR 3 through the render graph on a real GPU: the node runs offscreen on a camera set up the way
//! `apply::mod::drive_screen` sets up the screen camera (`post::set`, then `fsr3::set`), since the headless app has
//! no window. Needs a Vulkan device (`cargo test -p blackbox-bevy-render --features fsr3 --lib -- --ignored`). Like
//! `post_tests.rs`, the readback helpers are private to that file, so the small ones these tests need are repeated here.

use std::sync::{Arc, Mutex};

use bevy_app::{App, PostUpdate};
use bevy_asset::{Assets, Handle};
use bevy_camera::RenderTarget;
use bevy_ecs::entity::Entity;
use bevy_ecs::observer::On;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Commands, Res, RunSystemOnce};
use bevy_image::Image;
use bevy_render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use blackbox_gfx::{FrameParams, GraphicsApi, GraphicsSettings, RenderBackend, RgbaImage, UpscaleQuality, Upscaler};
use blackbox_gfx_testkit::camera::Camera;
use blackbox_gfx_testkit::{Mask, SceneId, compare};
use blackbox_gpu_passes::test_support::serial;
use glam::Vec3;
use wgpu::{TextureFormat, TextureUsages};

use crate::HeadlessBevy;
use crate::apply::camera as blackbox_camera;
use crate::ops::{BlackboxBridge, CameraSettings};

const SIZE: [u32; 2] = [160, 90];

fn fallback() -> bool {
    std::env::var("BLACKBOX_GPU_FALLBACK").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn bevy() -> Option<HeadlessBevy> {
    match HeadlessBevy::new(SIZE, GraphicsApi::Vulkan, fallback()) {
        Ok(renderer) => Some(renderer),
        Err(e) => {
            eprintln!("skipping: {e}");
            None
        }
    }
}

fn current_settings(backend: &HeadlessBevy) -> CameraSettings {
    backend.app().world().resource::<BlackboxBridge>().lock().settings
}

/// A fresh output-size render target, like `apply::capture::begin`'s.
fn target(app: &mut App) -> Handle<Image> {
    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let mut image = Image::new_target_texture(SIZE[0], SIZE[1], TextureFormat::Rgba8UnormSrgb, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    images.add(image)
}

/// A camera with `settings` applied the way the screen camera gets them every tick: the post components, then the
/// FSR 3 components. It targets an image so it can be read back, which the window-less app needs.
fn spawn_camera(app: &mut App, frame: &FrameParams, settings: &CameraSettings) -> (Entity, Handle<Image>) {
    let image = target(app);
    let bundle = blackbox_camera::bundle(frame, RenderTarget::Image(image.clone().into()), settings);
    let camera = app.world_mut().spawn(bundle).id();
    let settings = *settings;
    let camera_cut = frame.camera_cut;
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            blackbox_camera::set_post(&mut commands, camera, &settings);
            crate::fsr3::set(&mut commands, camera, &settings, camera_cut);
        })
        .expect("set_post and fsr3::set");
    (camera, image)
}

/// Readbacks waiting to be spawned by `drain_pending_readbacks` (see `post_tests.rs` for why they are queued).
#[derive(Resource, Clone, Default)]
struct PendingReadbacks(Arc<Mutex<Vec<PendingReadback>>>);

/// A readback this test still wants spawned, and where to file its bytes once it fires.
type PendingReadback = (Handle<Image>, Arc<Mutex<Option<Vec<u8>>>>);

fn drain_pending_readbacks(mut commands: Commands, pending: Res<PendingReadbacks>) {
    let items = std::mem::take(&mut *pending.0.lock().unwrap());
    for (image, done) in items {
        commands
            .spawn(ReadbackOnce::texture(image))
            .observe(move |read: On<ReadbackComplete>| *done.lock().unwrap() = Some(read.data.clone()));
    }
}

/// Ask for a readback of `image` at the next update, which makes the copy. Ask before the frame whose output should be
/// read: a readback requested after a frame copies a later frame.
fn request_readback(app: &mut App, image: &Handle<Image>) -> Arc<Mutex<Option<Vec<u8>>>> {
    if !app.world().contains_resource::<PendingReadbacks>() {
        app.init_resource::<PendingReadbacks>().add_systems(PostUpdate, drain_pending_readbacks);
    }
    let done: Arc<Mutex<Option<Vec<u8>>>> = Arc::default();
    app.world().resource::<PendingReadbacks>().0.lock().unwrap().push((image.clone(), done.clone()));
    done
}

/// Pump the app until the readback asked for by [`request_readback`] has arrived.
fn wait_readback(app: &mut App, done: &Arc<Mutex<Option<Vec<u8>>>>) -> RgbaImage {
    for _ in 0..60 {
        if let Some(data) = done.lock().unwrap().clone() {
            return unpad(&data, SIZE);
        }
        app.update();
    }
    panic!("the readback did not complete in time");
}

/// Read `image` back once, pumped until it resolves.
fn readback(app: &mut App, image: &Handle<Image>) -> RgbaImage {
    let done = request_readback(app, image);
    wait_readback(app, &done)
}

/// Unpad a readback's rows (the same maths as `apply::capture`).
fn unpad(data: &[u8], [width, height]: [u32; 2]) -> RgbaImage {
    let row = (width as usize) * 4;
    let stride = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
    let mut rgba = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        rgba.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    RgbaImage::new(width, height, rgba).expect("packed rows")
}

fn fsr3_settings(quality: UpscaleQuality) -> GraphicsSettings {
    GraphicsSettings { upscaler: Upscaler::Fsr3, upscale_quality: quality, ..GraphicsSettings::default() }
}

/// FSR 3 at Quality on a static scene: the upscaler runs on the render graph, and once the jitter cycle has run the
/// picture should settle, so a frame 16 later should barely differ from it.
#[test]
#[ignore = "needs a GPU"]
fn fsr3_converges_on_a_static_scene() {
    let _gpu = serial();
    let Some(mut bevy) = bevy() else { return };
    bevy.apply_graphics(&fsr3_settings(UpscaleQuality::Quality));
    let settings = current_settings(&bevy);
    assert_eq!(settings.upscaler, Upscaler::Fsr3, "the device must offer FSR 3 for this test");

    let built = SceneId::Grid.build(&mut bevy, SIZE);
    built.apply_layers(&mut bevy);
    let (_camera, image) = spawn_camera(bevy.app_mut(), &built.frame, &settings);

    // Settle past the jitter cycle (18 frames at 1.5x) before the first sample.
    for _ in 0..24 {
        bevy.render(&built.frame, &built.instances).expect("render");
    }
    let frame_n = readback(bevy.app_mut(), &image);
    for _ in 0..16 {
        bevy.render(&built.frame, &built.instances).expect("render");
    }
    let frame_n_plus_16 = readback(bevy.app_mut(), &image);
    built.release(&mut bevy);

    let diff = compare(&frame_n, &frame_n_plus_16, &Mask::all()).expect("same size");
    eprintln!("fsr3 convergence, frame N vs N+16: {diff}");
    assert!(diff.mean < 2.0 && diff.p99 <= 16, "a settled FSR 3 picture should barely change 16 frames later: {diff}");
}

/// The camera of frame `k` of the slide: 0.05 m sideways a frame, looking at the same place, so the whole world moves
/// across the picture and FSR 3 has motion to follow (the same look as the Grid scene's own camera).
fn slid(k: usize, base: &FrameParams) -> FrameParams {
    let x = 0.05 * k as f32;
    let aspect = SIZE[0] as f32 / SIZE[1] as f32;
    let fog = base.fog.as_ref().map(|f| (f.start, f.end));
    Camera::new(Vec3::new(x, -6.0, 4.5), Vec3::new(x, 14.0, 0.0)).frame(aspect, base.clear_color, fog)
}

/// The camera of the slide for the next update. It is set in `PostUpdate`, where `apply::apply` sets it in a window:
/// the previous view (which FSR 3 reprojects with) is taken in `PreUpdate`, so it must still be last frame's pose
/// when this frame's pose goes in. Set outside an update, the previous and current views would be the same.
#[derive(Resource)]
struct NextFrame {
    camera: Entity,
    frame: FrameParams,
    settings: CameraSettings,
}

fn follow_next_frame(mut commands: Commands, next: Res<NextFrame>) {
    blackbox_camera::follow(&mut commands, next.camera, &next.frame, &next.settings);
}

/// Draw `frames` frames of the slide with `settings` on a fresh renderer and return the last picture. The last
/// frame's readback is requested before that frame, so the picture is that frame's own.
fn slide(settings: &GraphicsSettings, frames: usize) -> Option<RgbaImage> {
    let mut bevy = bevy()?;
    bevy.apply_graphics(settings);
    let camera_settings = current_settings(&bevy);
    let built = SceneId::Grid.build(&mut bevy, SIZE);
    built.apply_layers(&mut bevy);
    let (camera, image) = spawn_camera(bevy.app_mut(), &built.frame, &camera_settings);
    bevy.app_mut().add_systems(PostUpdate, follow_next_frame);
    let mut last = None;
    for k in 0..frames {
        let frame = slid(k, &built.frame);
        bevy.app_mut().insert_resource(NextFrame { camera, frame, settings: camera_settings });
        last = (k + 1 == frames).then(|| request_readback(bevy.app_mut(), &image));
        bevy.render(&frame, &built.instances).expect("render");
    }
    let picture = last.map(|done| wait_readback(bevy.app_mut(), &done));
    built.release(&mut bevy);
    picture
}

/// FSR 3 on a moving camera against the native picture of the same camera (no upscaler, no anti-aliasing). Prints
/// the difference for FSR 3 at Quality and at the native ratio; it is a measurement, not a gate. Needs a GPU, and
/// `--nocapture` to see the numbers: `cargo test -p blackbox-bevy-render --features fsr3 --lib
/// fsr3_on_a_moving_camera -- --ignored --nocapture`.
#[test]
#[ignore = "needs a GPU; prints the moving-camera numbers"]
fn fsr3_on_a_moving_camera_tracks_the_native_picture() {
    let _gpu = serial();
    let frames = 30;
    let Some(native) = slide(&GraphicsSettings::default(), frames) else { return };
    for (label, quality) in [("quality", UpscaleQuality::Quality), ("native ratio", UpscaleQuality::Native)] {
        let Some(upscaled) = slide(&fsr3_settings(quality), frames) else { return };
        let diff = compare(&native, &upscaled, &Mask::all()).expect("same size");
        eprintln!("fsr3 moving camera, {label} vs native: {diff}");
    }
}
