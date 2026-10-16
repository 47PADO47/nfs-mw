//! The world material's prepass on a real GPU: the motion vectors are written for a moving object and are zero for a
//! still scene. Needs a Vulkan device (`cargo test -p blackbox-bevy-render --lib -- --ignored prepass`).
//!
//! The motion-vector prepass has no `COPY_SRC`, so the render world blits it into an `Rg16Float` image of the test's
//! own with Bevy's blit pipeline, and the test reads that image back the way `fsr3::gpu_tests` reads its frames.

use std::sync::{Arc, Mutex};

use bevy_app::{App, PostUpdate};
use bevy_asset::{AssetId, Assets, Handle};
use bevy_camera::RenderTarget;
use bevy_core_pipeline::blit::{BlitPipeline, BlitPipelineKey};
use bevy_core_pipeline::prepass::{
    DepthPrepass, MOTION_VECTOR_PREPASS_FORMAT, MotionVectorPrepass, ViewPrepassTextures,
};
use bevy_core_pipeline::{Core3d, Core3dSystems};
use bevy_ecs::entity::Entity;
use bevy_ecs::observer::On;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Commands, Res, ResMut};
use bevy_image::Image;
use bevy_render::RenderApp;
use bevy_render::gpu_readback::{ReadbackComplete, ReadbackOnce};
use bevy_render::render_asset::RenderAssets;
use bevy_render::render_resource::{
    Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, SpecializedRenderPipelines,
};
use bevy_render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy_render::texture::GpuImage;
use blackbox_gfx::{FrameParams, GraphicsApi, Instance, RenderBackend, RgbaImage};
use blackbox_gfx_testkit::{SceneId, camera::Camera};
use blackbox_gpu_passes::test_support::serial;
use glam::{Mat4, Vec3};
use wgpu::{TextureFormat, TextureUsages};

use crate::HeadlessBevy;
use crate::apply::camera as blackbox_camera;
use crate::ops::CameraSettings;

const SIZE: [u32; 2] = [160, 90];

/// The box in the Grid scene that moves (the third instance; see `blackbox-gfx-testkit`'s `scenes/grid.rs`).
const MOVING_BOX: usize = 2;

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

/// The image the render world writes the motion vectors into, by asset id (the render world holds no `Handle`).
#[derive(Resource, Clone, Copy)]
struct MotionTarget(AssetId<Image>);

/// Blit the test camera's motion-vector prepass into [`MotionTarget`]. Both ends are `Rg16Float`, so the readback
/// holds the motion vectors exactly as the material wrote them.
#[allow(clippy::too_many_arguments)]
fn blit_motion_vectors(
    target: Option<Res<MotionTarget>>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    blit: Res<BlitPipeline>,
    mut blits: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    view: ViewQuery<&ViewPrepassTextures>,
    mut ctx: RenderContext,
) {
    let Some(target) = target else { return };
    let Some(destination) = images.get(target.0) else { return };
    let Some(motion) = view.into_inner().motion_vectors.as_ref() else { return };
    let pipeline_id = blits.specialize(
        &cache,
        &blit,
        BlitPipelineKey {
            target_format: MOTION_VECTOR_PREPASS_FORMAT,
            blend_state: None,
            samples: 1,
            source_space: None,
        },
    );
    let Some(pipeline) = cache.get_render_pipeline(pipeline_id) else { return };
    let bind_group = blit.create_bind_group(&device, &motion.texture.default_view, &cache);
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("prepass test motion readback"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &destination.texture_view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}

/// Where a readback's bytes land once it completes.
type Done = Arc<Mutex<Option<Vec<u8>>>>;

/// Readbacks waiting to be spawned by `drain_pending_readbacks` (see `fsr3::gpu_tests` for why they are queued).
#[derive(Resource, Clone, Default)]
struct PendingReadbacks(Arc<Mutex<Vec<PendingReadback>>>);

/// A readback this test still wants spawned, and where to file its bytes once it fires.
type PendingReadback = (Handle<Image>, Done);

fn drain_pending_readbacks(mut commands: Commands, pending: Res<PendingReadbacks>) {
    let items = std::mem::take(&mut *pending.0.lock().unwrap());
    for (image, done) in items {
        commands
            .spawn(ReadbackOnce::texture(image))
            .observe(move |read: On<ReadbackComplete>| *done.lock().unwrap() = Some(read.data.clone()));
    }
}

/// Ask for a readback of `image` at the next update. The copy is made in that update, so ask before the frame whose
/// output should be read: a readback requested after a frame would copy a later, still frame's motion vectors.
fn request_readback(app: &mut App, image: &Handle<Image>) -> Done {
    if !app.world().contains_resource::<PendingReadbacks>() {
        app.init_resource::<PendingReadbacks>().add_systems(PostUpdate, drain_pending_readbacks);
    }
    let done: Done = Arc::default();
    app.world().resource::<PendingReadbacks>().0.lock().unwrap().push((image.clone(), done.clone()));
    done
}

/// Pump the app until the readback asked for by [`request_readback`] has arrived.
fn wait_readback(app: &mut App, done: &Done) -> RgbaImage {
    for _ in 0..60 {
        if let Some(data) = done.lock().unwrap().clone() {
            return unpad(&data, SIZE);
        }
        app.update();
    }
    panic!("the readback did not complete in time");
}

/// Render one frame and count the motion vectors it wrote.
fn render_and_count(
    bevy: &mut HeadlessBevy,
    motion: &Handle<Image>,
    frame: &FrameParams,
    instances: &[Instance],
) -> usize {
    let done = request_readback(bevy.app_mut(), motion);
    bevy.render(frame, instances).expect("render");
    moving_texels(&wait_readback(bevy.app_mut(), &done))
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

/// How many texels hold a non-zero motion vector. An `Rg16Float` texel is four bytes, two halves; a half with only
/// its sign bit set (`-0.0`) counts as zero.
fn moving_texels(image: &RgbaImage) -> usize {
    image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|texel| {
            let red = u16::from_le_bytes([texel[0], texel[1]]) & 0x7fff;
            let green = u16::from_le_bytes([texel[2], texel[3]]) & 0x7fff;
            red != 0 || green != 0
        })
        .count()
}

/// The image the motion vectors are copied into: `Rg16Float`, readable.
fn motion_image(app: &mut App) -> Handle<Image> {
    let mut images = app.world_mut().resource_mut::<Assets<Image>>();
    let mut image = Image::new_target_texture(SIZE[0], SIZE[1], MOTION_VECTOR_PREPASS_FORMAT, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    images.add(image)
}

/// A camera with the depth and motion-vector prepasses, drawing into a colour target of its own, and the render-world
/// blit that copies its motion vectors into `motion`.
fn spawn_camera(app: &mut App, frame: &FrameParams, motion: &Handle<Image>) -> Entity {
    let colour = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        images.add(Image::new_target_texture(SIZE[0], SIZE[1], TextureFormat::Rgba8UnormSrgb, None))
    };
    let bundle = blackbox_camera::bundle(frame, RenderTarget::Image(colour.into()), &CameraSettings::default());
    let camera = app.world_mut().spawn((bundle, DepthPrepass, MotionVectorPrepass)).id();
    app.insert_resource(NextFrame { camera, frame: *frame }).add_systems(PostUpdate, follow_next_frame);
    app.get_sub_app_mut(RenderApp)
        .expect("the render app")
        .insert_resource(MotionTarget(motion.id()))
        .add_systems(Core3d, blit_motion_vectors.in_set(Core3dSystems::EarlyPostProcess));
    camera
}

/// The camera's frame for the next update. It is set in `PostUpdate`, where `apply::apply` sets it in a window: the
/// previous view is taken in `PreUpdate`, so it must still be last frame's pose when this frame's pose goes in.
#[derive(Resource)]
struct NextFrame {
    camera: Entity,
    frame: FrameParams,
}

fn follow_next_frame(mut commands: Commands, next: Res<NextFrame>) {
    blackbox_camera::follow(&mut commands, next.camera, &next.frame, &CameraSettings::default());
}

/// The Grid scene's camera at frame `k` of a slide: 0.05 m sideways a frame, looking at the same place.
fn slid(k: usize, base: &FrameParams) -> FrameParams {
    let x = 0.05 * k as f32;
    let aspect = SIZE[0] as f32 / SIZE[1] as f32;
    let fog = base.fog.as_ref().map(|f| (f.start, f.end));
    Camera::new(Vec3::new(x, -6.0, 4.5), Vec3::new(x, 14.0, 0.0)).frame(aspect, base.clear_color, fog)
}

/// The Grid scene's instances, with the moving box moved `step` metres along x.
fn with_box_moved(instances: &[Instance], step: f32) -> Vec<Instance> {
    let mut moved = instances.to_vec();
    let original = moved[MOVING_BOX].transform;
    moved[MOVING_BOX].transform = Mat4::from_translation(Vec3::new(step, 0.0, 0.0)) * original;
    moved
}

/// Motion vectors are written on a moving box and are zero on a still scene. The first frames are skipped: the
/// previous view and the previous transforms are only set up from the second frame on.
#[test]
#[ignore = "needs a GPU"]
fn motion_vectors_are_written_for_a_moving_box_and_are_zero_for_a_still_scene() {
    let _gpu = serial();
    let Some(mut bevy) = bevy() else { return };
    let built = SceneId::Grid.build(&mut bevy, SIZE);
    built.apply_layers(&mut bevy);
    let motion = motion_image(bevy.app_mut());
    spawn_camera(bevy.app_mut(), &built.frame, &motion);

    for _ in 0..3 {
        render_and_count(&mut bevy, &motion, &built.frame, &built.instances);
    }
    let still = render_and_count(&mut bevy, &motion, &built.frame, &built.instances);

    let mut moving = 0;
    for step in [0.2_f32, 0.4, 0.6, 0.8] {
        let instances = with_box_moved(&built.instances, step);
        moving = render_and_count(&mut bevy, &motion, &built.frame, &instances);
    }
    built.release(&mut bevy);

    eprintln!("prepass motion: still {still} texels, moving box {moving} texels of {}", SIZE[0] * SIZE[1]);
    assert_eq!(still, 0, "a still scene must have no motion vectors");
    assert!(moving > 0, "a moving box must write motion vectors");
}

/// Motion vectors are written when the camera moves over a still scene: every surface shifts across the picture.
#[test]
#[ignore = "needs a GPU"]
fn motion_vectors_are_written_for_a_moving_camera() {
    let _gpu = serial();
    let Some(mut bevy) = bevy() else { return };
    let built = SceneId::Grid.build(&mut bevy, SIZE);
    built.apply_layers(&mut bevy);
    let motion = motion_image(bevy.app_mut());
    let camera = spawn_camera(bevy.app_mut(), &built.frame, &motion);

    let mut moving = 0;
    for k in 0..6 {
        bevy.app_mut().insert_resource(NextFrame { camera, frame: slid(k, &built.frame) });
        moving = render_and_count(&mut bevy, &motion, &built.frame, &built.instances);
    }
    built.release(&mut bevy);

    eprintln!("prepass motion, moving camera: {moving} texels of {}", SIZE[0] * SIZE[1]);
    assert!(moving > 0, "a moving camera must write motion vectors");
}
