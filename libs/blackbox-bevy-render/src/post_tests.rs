//! The four tests `pr-list.md` asks PR 10 for: TAA converges on a static scene, a camera cut resets its
//! history, no prepass components exist when TAA is off, and FSR 1 matches native at 67%. Needs a GPU
//! (`cargo test -p blackbox-bevy-render -- --include-ignored`), like `parity_tests.rs`.
//!
//! The first three drive a camera directly (`spawn_camera`, `readback`), bypassing the facade's screen
//! camera: that camera only exists behind a real window (`apply::mod::drive_screen`'s
//! `windows.single_mut()`), which a headless `App` never has, and the one-shot `request_capture` camera
//! despawns itself a few frames after it is requested (`apply::capture`) — too short-lived to accumulate
//! 16 frames of TAA history and still be there to read back a second time. A camera spawned here directly
//! is driven by the exact same production code (`apply::camera::bundle`/`set_post`/`follow`) and persists
//! for as long as the test wants.

use std::sync::{Arc, Mutex};

use bevy_anti_alias::taa::TemporalAntiAliasing;
use bevy_app::{App, PostUpdate};
use bevy_asset::{Assets, Handle};
use bevy_camera::RenderTarget;
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy_ecs::entity::Entity;
use bevy_ecs::observer::On;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Commands, Res, RunSystemOnce};
use bevy_image::Image;
use bevy_render::camera::TemporalJitter;
use bevy_render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use blackbox_gfx::{Antialiasing, FrameParams, GraphicsApi, GraphicsSettings, RenderBackend, RgbaImage, Upscaler};
use blackbox_gfx_testkit::camera::Camera;
use blackbox_gfx_testkit::{Mask, SceneId, Tolerance, capture_scene, compare};
use blackbox_gpu_passes::test_support::serial;
use glam::Vec3;
use wgpu::{TextureFormat, TextureUsages};

use crate::HeadlessBevy;
use crate::apply::camera as blackbox_camera;
use crate::ops::{BlackboxBridge, CameraSettings};

const SIZE: [u32; 2] = [160, 90];
/// A camera this far into the grid scene sees different rows and tiles than the one its own builder
/// places, so a "ghost" of one pose blended into the other's picture would show up as a large difference.
const POSE_B_EYE: Vec3 = Vec3::new(0.0, 40.0, 4.5);
const POSE_B_TARGET: Vec3 = Vec3::new(0.0, 54.0, 0.0);

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

fn native(size: [u32; 2]) -> Option<blackbox_render::Renderer> {
    let options = blackbox_render::RendererOptions {
        backend: GraphicsApi::Vulkan,
        vsync: false,
        force_fallback_adapter: fallback(),
    };
    match blackbox_render::Renderer::headless((size[0], size[1]), options) {
        Ok(renderer) => Some(renderer),
        Err(e) => {
            eprintln!("skipping: {e}");
            None
        }
    }
}

/// The settings `apply_graphics` last resolved, straight from the bridge both the facade and the render
/// world's own systems read.
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

/// Spawn a camera at `frame` with `settings` already applied (bloom, tone mapping, TAA, FSR 1, ...),
/// targeting a fresh image it is never asked to give up: unlike a capture's own camera, this one is
/// driven, and can be read back, for as many frames as the test wants.
fn spawn_camera(app: &mut App, frame: &FrameParams, settings: &CameraSettings) -> (Entity, Handle<Image>) {
    let image = target(app);
    let bundle = blackbox_camera::bundle(frame, RenderTarget::Image(image.clone().into()), settings);
    let camera = app.world_mut().spawn(bundle).id();
    let settings = *settings;
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            blackbox_camera::set_post(&mut commands, camera, &settings);
        })
        .expect("set_post");
    (camera, image)
}

/// Move `camera` to `frame` (a new pose, possibly a cut), the same per-frame update the screen camera
/// gets from `apply::mod::drive_screen` every tick.
fn follow(app: &mut App, camera: Entity, frame: &FrameParams, settings: &CameraSettings) {
    let frame = *frame;
    let settings = *settings;
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            blackbox_camera::follow(&mut commands, camera, &frame, &settings);
        })
        .expect("follow");
}

/// A readback this test still wants spawned, and where to file its bytes once it fires.
type PendingReadback = (Handle<Image>, Arc<Mutex<Option<Vec<u8>>>>);

/// Readbacks waiting to be spawned by [`drain_pending_readbacks`].
///
/// Bevy's `cleanup_readback_once` (`bevy_render::gpu_readback`) runs in `First`, every tick, and strips
/// `Readback`/`ReadbackOnce` off every entity that has them — including one spawned directly on the
/// `World` between two `app.update()` calls, since that entity exists *before* `First` even runs for the
/// next tick, so it gets stripped there before the render world's `ExtractSchedule` (which runs near the
/// end of the same tick) ever sees it: zero readback attempts, the event never fires. Queuing the spawn
/// here and draining it from a system in `PostUpdate` (after `First`, before the render sub-app's own
/// update) gives it exactly the timing `apply::capture::begin`'s `commands.spawn(...)` already has for
/// free by running inside the facade's own `PostUpdate` system.
#[derive(Resource, Clone, Default)]
struct PendingReadbacks(Arc<Mutex<Vec<PendingReadback>>>);

fn drain_pending_readbacks(mut commands: Commands, pending: Res<PendingReadbacks>) {
    let items = std::mem::take(&mut *pending.0.lock().unwrap());
    for (image, done) in items {
        commands
            .spawn(ReadbackOnce::texture(image))
            .observe(move |read: On<ReadbackComplete>| *done.lock().unwrap() = Some(read.data.clone()));
    }
}

/// Read `image` back once, pumped until it resolves. Nothing here despawns the camera or the image,
/// unlike `apply::capture`'s one-shot flow.
fn readback(app: &mut App, image: &Handle<Image>) -> RgbaImage {
    if !app.world().contains_resource::<PendingReadbacks>() {
        app.init_resource::<PendingReadbacks>().add_systems(PostUpdate, drain_pending_readbacks);
    }
    let done: Arc<Mutex<Option<Vec<u8>>>> = Arc::default();
    app.world().resource::<PendingReadbacks>().0.lock().unwrap().push((image.clone(), done.clone()));
    for _ in 0..60 {
        if let Some(data) = done.lock().unwrap().clone() {
            return unpad(&data, SIZE);
        }
        app.update();
    }
    panic!("the readback did not complete in time");
}

/// Unpad a readback's rows, like `apply::capture`'s (private there; the same bytes, so the same maths).
fn unpad(data: &[u8], [width, height]: [u32; 2]) -> RgbaImage {
    let row = (width as usize) * 4;
    let stride = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
    let mut rgba = Vec::with_capacity(row * height as usize);
    for y in 0..height as usize {
        rgba.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    RgbaImage::new(width, height, rgba).expect("packed rows")
}

fn taa_settings() -> GraphicsSettings {
    GraphicsSettings {
        post: blackbox_gfx::PostSettings { antialiasing: Antialiasing::Taa, ..Default::default() },
        ..Default::default()
    }
}

#[test]
#[ignore = "needs a GPU"]
fn taa_converges_on_a_static_scene() {
    let _gpu = serial();
    let Some(mut bevy) = bevy() else { return };
    bevy.apply_graphics(&taa_settings());

    let built = SceneId::Grid.build(&mut bevy, SIZE);
    built.apply_layers(&mut bevy);
    let settings = current_settings(&bevy);
    let (_camera, image) = spawn_camera(bevy.app_mut(), &built.frame, &settings);

    // Settle past the Halton sequence's own period (8 frames) before taking the first sample, so both
    // samples are in steady state and the comparison is about convergence, not the initial transient.
    for _ in 0..16 {
        bevy.render(&built.frame, &built.instances).expect("render");
    }
    let frame_n = readback(bevy.app_mut(), &image);
    for _ in 0..16 {
        bevy.render(&built.frame, &built.instances).expect("render");
    }
    let frame_n_plus_16 = readback(bevy.app_mut(), &image);
    built.release(&mut bevy);

    let diff = compare(&frame_n, &frame_n_plus_16, &Mask::all()).expect("same size");
    eprintln!("taa convergence, frame N vs N+16: {diff}");
    // TAA keeps jittering every frame even once the history has settled, so a few edge pixels (the
    // checkerboard seams, the boxes' silhouettes) keep a little temporal dither forever: `max` alone is
    // not a convergence signal. `mean` is: almost every pixel should already agree closely.
    assert!(diff.mean < 2.0 && diff.p99 <= 16, "a converged static scene should barely change 16 frames later: {diff}");
}

#[test]
#[ignore = "needs a GPU"]
fn camera_cut_resets_taa_history_so_no_ghost_remains() {
    let _gpu = serial();
    let Some(mut bevy) = bevy() else { return };
    bevy.apply_graphics(&taa_settings());

    let built = SceneId::Grid.build(&mut bevy, SIZE);
    built.apply_layers(&mut bevy);
    let settings = current_settings(&bevy);
    let (camera, image) = spawn_camera(bevy.app_mut(), &built.frame, &settings);

    // Settle a history at pose A.
    for _ in 0..16 {
        bevy.render(&built.frame, &built.instances).expect("render");
    }

    // Teleport to a very different pose, `camera_cut` set in the same frame.
    let pose_b = Camera::new(POSE_B_EYE, POSE_B_TARGET);
    let mut cut = built.frame;
    cut.view = pose_b.view();
    cut.camera_position = pose_b.eye;
    cut.camera_cut = true;
    follow(bevy.app_mut(), camera, &cut, &settings);
    bevy.render(&cut, &built.instances).expect("render the cut frame");
    let after_cut = readback(bevy.app_mut(), &image);

    // An unjittered camera (no TAA at all) at the same pose is the ground truth a reset frame should be
    // close to. Comparing against another TAA camera instead would confound this with jitter: two
    // "reset" frames rendered a tick apart land on different Halton phases, which on a scene this
    // detailed (a dense checkerboard) shifts enough edges to look like a much bigger difference than
    // ghosting ever would on its own, even with no bug at all.
    let plain = CameraSettings { post: blackbox_gfx::PostSettings::default(), ..settings };
    let (_reference, reference_image) = spawn_camera(bevy.app_mut(), &cut, &plain);
    bevy.render(&cut, &built.instances).expect("render the reference frame");
    let reference = readback(bevy.app_mut(), &reference_image);
    built.release(&mut bevy);

    let diff = compare(&after_cut, &reference, &Mask::all()).expect("same size");
    eprintln!("camera cut vs an unjittered camera at the same pose: {diff}");
    // Threshold between what a reset frame's own small jitter-driven edge noise accounts for (close to
    // `taa_converges_on_a_static_scene`'s own numbers) and what a failed reset would look like: pose A
    // blended in at up to 90% (`DEFAULT_HISTORY_BLEND_RATE` in `bevy_anti_alias`'s TAA shader), which on
    // two poses this different would push the mean into the double digits, not a handful of edge pixels.
    assert!(
        diff.mean < 3.0,
        "a reset history should look like a fresh, unjittered frame, not a blend with pose A: {diff}"
    );
}

#[test]
#[ignore = "needs a GPU"]
fn no_prepass_components_exist_when_taa_is_off() {
    let _gpu = serial();
    let Some(mut bevy) = bevy() else { return };
    bevy.apply_graphics(&GraphicsSettings::default());
    let frame = Camera::new(Vec3::new(0.0, -6.0, 4.5), Vec3::new(0.0, 14.0, 0.0)).frame(1.0, [0.0; 3], None);
    let settings = current_settings(&bevy);
    let (camera, _image) = spawn_camera(bevy.app_mut(), &frame, &settings);
    bevy.app_mut().update();
    assert_no_taa_or_prepass(bevy.app_mut(), camera);

    // Turn it on, then off again, so the removal is proven to actually strip what it once added, not
    // just that a camera that never had it lacks it.
    bevy.apply_graphics(&taa_settings());
    let on = current_settings(&bevy);
    {
        let app = bevy.app_mut();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                blackbox_camera::set_post(&mut commands, camera, &on);
            })
            .expect("set_post on");
        app.update();
    }
    assert!(bevy.app().world().get::<TemporalAntiAliasing>(camera).is_some(), "TAA should be on now");
    assert!(bevy.app().world().get::<DepthPrepass>(camera).is_some(), "and its prepass with it");

    bevy.apply_graphics(&GraphicsSettings::default());
    let off = current_settings(&bevy);
    {
        let app = bevy.app_mut();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                blackbox_camera::set_post(&mut commands, camera, &off);
            })
            .expect("set_post off");
        app.update();
    }
    assert_no_taa_or_prepass(bevy.app_mut(), camera);
}

fn assert_no_taa_or_prepass(app: &mut App, camera: Entity) {
    let world = app.world();
    assert!(world.get::<TemporalAntiAliasing>(camera).is_none(), "no TemporalAntiAliasing when TAA is off");
    assert!(world.get::<DepthPrepass>(camera).is_none(), "no DepthPrepass when TAA is off");
    assert!(world.get::<MotionVectorPrepass>(camera).is_none(), "no MotionVectorPrepass when TAA is off");
    assert!(world.get::<TemporalJitter>(camera).is_none(), "no TemporalJitter when TAA is off");
}

/// `fsr1_active`'s whole point is a spatial upscale after an anti-aliased, display-referred image; both
/// renderers run the exact same `blackbox_gpu_passes::fsr1` shader on it, so this should track the other
/// strict-fidelity scenes closely, not the wider "different algorithm by design" tolerance post effects
/// otherwise get (`docs/plans/gfx-renderers/verification-risks.md`, §7).
#[test]
#[ignore = "needs a GPU"]
fn fsr1_matches_native_within_tolerance_at_67_percent() {
    let _gpu = serial();
    let settings = GraphicsSettings { upscaler: Upscaler::Fsr1, render_scale: 0.67, ..GraphicsSettings::default() };

    let Some(mut native) = native(SIZE) else { return };
    native.apply_graphics(&settings);
    let a = capture_scene(&mut native, SceneId::Grid, SIZE).expect("native capture");
    drop(native);

    let Some(mut bevy) = bevy() else { return };
    bevy.apply_graphics(&settings);
    let b = capture_scene(&mut bevy, SceneId::Grid, SIZE).expect("bevy capture");

    let diff = compare(&a, &b, &Mask::all()).expect("same size");
    eprintln!("fsr1 at 67%: {diff}");
    // Measured mean 1.78, p99 18, max 54 (Iris Xe, Vulkan): wider than the Strict scenes' own tolerance
    // (mean 0.5) because EASU and RCAS are edge-adaptive, so the small pre-upscale differences every
    // scene already has (`the_grid_matches_native`'s own mean ~0.03) get amplified at the checkerboard's
    // many edges, not because the upscale pass itself differs — both renderers run the exact same
    // `blackbox_gpu_passes::fsr1` shader on it.
    Tolerance::new(64, 3.0, 24).check(&diff).unwrap_or_else(|e| panic!("{e}"));
}
