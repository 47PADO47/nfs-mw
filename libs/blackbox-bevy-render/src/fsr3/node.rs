//! The render-world side of FSR 3: the jitter that goes into the projection, and the upscale that takes the place of
//! TAA in `Core3dSystems::EarlyPostProcess`.
//!
//! The upscaler writes its output to a storage texture at the display size. The view's main texture is neither
//! storage-capable nor copyable into (`CameraMainTextureUsages` is render attachment, texture binding and copy source),
//! so the output is then blitted into the view target's next texture (`post_process_write`), and the bloom and tone
//! mapping that follow read it from there.

use std::time::Instant;

use bevy_core_pipeline::blit::{BlitPipeline, BlitPipelineKey};
use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_core_pipeline::{Core3d, Core3dSystems};
use bevy_diagnostic::FrameCount;
use bevy_ecs::query::With;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Query, Res, ResMut};
use bevy_render::camera::{ExtractedCamera, TemporalJitter};
use bevy_render::render_resource::{
    Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor, SpecializedRenderPipelines, TextureView,
};
use bevy_render::renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy_render::view::{ExtractedView, ViewTarget};
use bevy_render::{ExtractSchedule, Render, RenderSystems};
use fsr3_wgpu::{Fsr3Config, Fsr3Context, Fsr3Inputs, Fsr3Outputs};
use glam::{UVec2, Vec2};

use super::component::Fsr3Upscale;
use super::sizes;
use crate::ops::BlackboxBridge;
use crate::systems::effects;

/// The upscaler and its output, made on first use. Lives in the render world; the history is in `context`.
#[derive(Resource, Default)]
pub struct Fsr3Render {
    context: Option<Fsr3Context>,
    /// Set once the context could not be made, so the error is logged once and the pass stops.
    unavailable: bool,
    /// The display-size storage texture the upscaler writes, with the size it was made for.
    output: Option<(UVec2, wgpu::TextureView)>,
    last_frame: Option<Instant>,
}

impl Fsr3Render {
    /// Seconds since the previous upscale (0 for the first one), for the upscaler's time-based terms.
    fn tick(&mut self, now: Instant) -> f32 {
        let delta = self.last_frame.map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        self.last_frame = Some(now);
        delta
    }

    /// The context and the output texture for a display of `display` pixels, made on first use. `None` when the
    /// context cannot be made (an error is logged once). `adapter` must be the one `device` was made from.
    fn parts(
        &mut self,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        display: [u32; 2],
    ) -> Option<(&mut Fsr3Context, wgpu::TextureView)> {
        if self.unavailable {
            return None;
        }
        if self.context.is_none() {
            match Fsr3Context::new_for_adapter(adapter, device, Fsr3Config::new()) {
                Ok(context) => self.context = Some(context),
                Err(error) => {
                    log::error!("fsr3: the upscaler could not be created: {error}");
                    self.unavailable = true;
                    return None;
                }
            }
        }
        let size = UVec2::from_array(display);
        if !matches!(&self.output, Some((made, _)) if *made == size) {
            self.output = Some((size, output_texture(device, display)));
        }
        let view = self.output.as_ref()?.1.clone();
        Some((self.context.as_mut()?, view))
    }
}

/// The display-size texture the upscaler writes: storage for it, texture binding for the blit that follows.
fn output_texture(device: &wgpu::Device, [width, height]: [u32; 2]) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("blackbox fsr3 output"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: Fsr3Config::new().output_format,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The jitter of this frame, in the same units as Bevy's own TAA jitter: `offset` in render pixels, which Bevy turns
/// into the projection (`TemporalJitter::jitter_projection`). Runs with the other view preparation, so the projection
/// it feeds is the one the prepass and the main pass draw with.
pub fn prepare_jitter(
    frame_count: Res<FrameCount>,
    bridge: Res<BlackboxBridge>,
    mut cameras: Query<(&ExtractedCamera, &mut TemporalJitter), With<Fsr3Upscale>>,
) {
    let render_scale = bridge.lock().settings.render_scale;
    for (camera, mut jitter) in &mut cameras {
        let Some(display) = camera.physical_target_size else { continue };
        let display = [display.x, display.y];
        let render = sizes::render_size(display, render_scale);
        jitter.offset = bevy_math::Vec2::from_array(sizes::jitter(frame_count.0, render, display));
    }
}

/// Upscale the scene from the render size to the display size, in place of TAA, and write the result into the view
/// target for the post-processing that follows (bloom, tone mapping).
///
/// Runs after the world effect layer (`effects_soft_pass`), so the particles and glow are in the colour the upscaler
/// reads. The camera's own `ExtractedView` gives the near plane and field of view (`sizes::near_and_fov_y`).
#[allow(clippy::too_many_arguments)]
pub fn fsr3_pass(
    mut render: ResMut<Fsr3Render>,
    bridge: Res<BlackboxBridge>,
    device: Res<RenderDevice>,
    adapter: Res<RenderAdapter>,
    queue: Res<RenderQueue>,
    blit: Res<BlitPipeline>,
    mut blits: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    cache: Res<PipelineCache>,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &ViewTarget,
        &ViewPrepassTextures,
        &TemporalJitter,
        &Fsr3Upscale,
    )>,
    mut ctx: RenderContext,
) {
    let (camera, extracted, target, prepass, jitter, upscale) = view.into_inner();
    let Some(display) = camera.physical_target_size else { return };
    let display = [display.x, display.y];
    let (Some(depth), Some(motion)) = (&prepass.depth, &prepass.motion_vectors) else { return };
    let Some((camera_near, camera_fov_y)) = sizes::near_and_fov_y(extracted.clip_from_view) else { return };

    let blit_id = blits.specialize(
        &cache,
        &blit,
        BlitPipelineKey {
            target_format: target.main_texture_format(),
            blend_state: None,
            samples: 1,
            source_space: None,
        },
    );
    // Checked before anything runs, so a frame never has the upscale done and the blit not: the blit pipeline is
    // compiled synchronously in this renderer (`synchronous_pipeline_compilation`), so this only matters on the
    // first frame.
    let Some(blit_pipeline) = cache.get_render_pipeline(blit_id) else { return };

    let (render_scale, upscale_sharpness) = {
        let shared = bridge.lock();
        (shared.settings.render_scale, shared.settings.upscale_sharpness)
    };
    let render_size = sizes::render_size(display, render_scale);
    let delta_time = render.tick(Instant::now());
    let Some((context, output)) = render.parts(&adapter, device.wgpu_device(), display) else { return };

    let inputs = Fsr3Inputs {
        color: target.main_texture_view(),
        depth: &depth.texture.default_view,
        motion_vectors: &motion.texture.default_view,
        exposure: None,
        reactive: None,
        transparency_and_composition: None,
        render_size: UVec2::from_array(render_size),
        jitter: Vec2::from_array(jitter.offset.to_array()),
        motion_vector_scale: Vec2::from_array(sizes::motion_vector_scale(render_size)),
        delta_time,
        pre_exposure: 0.0,
        sharpness: sizes::sharpness(upscale_sharpness),
        camera_near,
        // Ignored for an infinite far plane, which Bevy's reverse-Z projection always is.
        camera_far: f32::INFINITY,
        camera_fov_y,
        view_space_to_meters: 1.0,
        reset: upscale.reset,
    };
    let outputs = Fsr3Outputs { output: &output, size: UVec2::from_array(display) };
    if let Err(error) = context.dispatch(device.wgpu_device(), &queue, ctx.command_encoder(), &inputs, &outputs) {
        log::error!("fsr3: the upscale could not be recorded: {error}");
        return;
    }

    let bind_group = blit.create_bind_group(&device, &TextureView::from(output), &cache);
    let post = target.post_process_write();
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("fsr3 output"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_render_pipeline(blit_pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);
}

/// The systems `install` adds: the marker's extraction, the jitter with the view preparation, and the upscale after
/// the world effects.
pub fn add_systems(render_app: &mut bevy_app::SubApp) {
    render_app
        .init_resource::<Fsr3Render>()
        .add_systems(ExtractSchedule, super::component::extract)
        .add_systems(Render, prepare_jitter.in_set(RenderSystems::PrepareViews))
        .add_systems(Core3d, fsr3_pass.in_set(Core3dSystems::EarlyPostProcess).after(effects::effects_soft_pass));
}
