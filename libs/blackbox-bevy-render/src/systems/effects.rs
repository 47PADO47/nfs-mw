//! The world effect layer: surfaces, particles, streaks, glows and textured effects, drawn through
//! `blackbox_gpu_passes::Effects` in the same position native draws them (`gpu/frame.rs`): inside the
//! scene's own pass, after the world meshes, before any post-processing; detailed particles in a second
//! pass over the finished colour and depth. Both systems run `.in_set(Core3dSystems::EarlyPostProcess)`,
//! chained, which is before `tonemapping`'s `PostProcess` set.

use std::collections::HashMap;

use bevy_asset::Handle;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Commands, Res, ResMut};
use bevy_image::Image;
use bevy_render::Extract;
use bevy_render::render_asset::RenderAssets;
use bevy_render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy_render::texture::GpuImage;
use bevy_render::view::{ExtractedView, ViewDepthStencilTexture, ViewTarget};
use blackbox_gpu_passes::{Effects, Globals, SoftDraw, WorldBindings};

use crate::apply::WorldState;
use crate::material::Params;
use crate::ops::{BlackboxBridge, EffectCapacities};

/// The texture map and fog/light parameters `WorldState` holds, extracted into the render world each
/// frame (unlike effects and the UI layer, which read [`BlackboxBridge`] directly: these come from
/// `WorldState`, which only the main world has).
#[derive(Resource, Default)]
pub struct ExtractedWorld {
    pub textures: HashMap<usize, Handle<Image>>,
    pub params: Params,
}

pub fn extract_world(mut commands: Commands, state: Extract<Res<WorldState>>) {
    commands
        .insert_resource(ExtractedWorld { textures: state.textures.clone(), params: state.params.unwrap_or_default() });
}

/// Owns the pipelines and reused buffers, created once the device exists (the first frame a view runs
/// in `EarlyPostProcess`).
#[derive(Resource, Default)]
pub struct EffectsRender(Option<Inner>);

struct Inner {
    bindings: WorldBindings,
    effects: Effects,
    /// Bind groups for this frame's textured effects, rebuilt every frame (the layer is usually a
    /// handful of textures; nothing here is reused across frames). Built with the raw `wgpu::Device`
    /// against `bindings.texture_layout`, since `Effects::draw_textured` takes raw `wgpu::BindGroup`s.
    textured: HashMap<usize, wgpu::BindGroup>,
}

impl EffectsRender {
    fn get_or_init(&mut self, device: &RenderDevice, format: wgpu::TextureFormat) -> &mut Inner {
        self.0.get_or_insert_with(|| {
            let bindings = WorldBindings::new(device.wgpu_device());
            let effects = Effects::new(device.wgpu_device(), format, &bindings);
            Inner { bindings, effects, textured: HashMap::new() }
        })
    }
}

/// Effect vertices are world-space, not model-local, so (unlike mesh instances, converted by their
/// transform in `apply/instances.rs`) nothing else puts them in Bevy's axes before they reach the GPU.
fn to_bevy_axes(layer: &mut blackbox_gfx::EffectLayer) {
    let convert = |v: &mut blackbox_gfx::EffectVertex| {
        v.position = crate::axes::point(glam::Vec3::from(v.position)).to_array();
    };
    for batch in [&mut layer.surfaces, &mut layer.particles, &mut layer.streaks, &mut layer.glows] {
        batch.iter_mut().for_each(convert);
    }
    for textured in &mut layer.textured {
        textured.vertices.iter_mut().for_each(convert);
    }
}

/// The `Globals` block every world pass reads: the camera and the fog, like
/// [`crate::material::Params::of`] builds for the material, plus the render-scale mip bias.
fn globals_of(view: &ExtractedView, params: &Params, mip_bias: f32) -> Globals {
    let view_proj =
        view.clip_from_world.unwrap_or_else(|| view.clip_from_view * view.world_from_view.to_matrix().inverse());
    let camera_pos = view.world_from_view.translation();
    Globals {
        view_proj: view_proj.to_cols_array_2d(),
        camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z, 1.0],
        light_dir: [0.0; 4],
        fog_color: params.fog_color.to_array(),
        fog_range: [params.fog_range.x, params.fog_range.y, mip_bias, 0.0],
    }
}

/// Draws the surfaces, particles (unless detailed), streaks, glows and textured effects inside the
/// scene's own pass (load the colour and depth the world meshes just wrote).
#[allow(clippy::too_many_arguments)]
pub fn effects_main_pass(
    mut render: ResMut<EffectsRender>,
    bridge: Res<BlackboxBridge>,
    extracted: Res<ExtractedWorld>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    view: ViewQuery<(&ExtractedView, &ViewTarget, &ViewDepthStencilTexture)>,
    mut ctx: RenderContext,
) {
    let (extracted_view, target, depth) = view.into_inner();
    let format = target.main_texture_format();
    let (mut layer, mip_bias) = {
        let shared = bridge.lock();
        (shared.effects.clone(), shared.settings.mip_bias)
    };
    to_bevy_axes(&mut layer);

    let inner = render.get_or_init(&device, format);
    inner.effects.use_format(device.wgpu_device(), format);

    {
        let mut shared = bridge.lock();
        if shared.effect_capacities.is_none() {
            shared.effect_capacities = Some(EffectCapacities {
                surfaces: inner.effects.surface_capacity(),
                particles: inner.effects.particle_capacity(),
                streaks: inner.effects.streak_capacity(),
            });
        }
    }

    inner.effects.upload(device.wgpu_device(), &queue, &layer);

    inner.textured.clear();
    for textured in &layer.textured {
        let raw = textured.texture.raw();
        if inner.textured.contains_key(&raw) {
            continue;
        }
        let Some(handle) = extracted.textures.get(&raw) else { continue };
        let Some(gpu_image) = images.get(handle) else { continue };
        let bind_group = device.wgpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blackbox effect texture"),
            layout: &inner.bindings.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&gpu_image.texture_view),
                },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&gpu_image.sampler) },
            ],
        });
        inner.textured.insert(raw, bind_group);
    }

    let globals = globals_of(extracted_view, &extracted.params, mip_bias);
    queue.write_buffer(&inner.bindings.globals, 0, bytemuck::bytes_of(&globals));

    let encoder = ctx.command_encoder();
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("blackbox effects"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: Some(depth.get_attachment(wgpu::StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_bind_group(0, &inner.bindings.globals_bind_group, &[]);
    inner.effects.draw(&mut pass);
    let textured = &inner.textured;
    inner.effects.draw_textured(&mut pass, |h| textured.get(&h.raw()));
}

/// Draws the detailed particles over the finished colour, fading them against the finished depth.
/// Does nothing before [`effects_main_pass`] has created the pipelines, or when nothing is detailed.
pub fn effects_soft_pass(
    mut render: ResMut<EffectsRender>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    view: ViewQuery<(&ExtractedView, &ViewTarget, &ViewDepthStencilTexture)>,
    mut ctx: RenderContext,
) {
    let Some(inner) = render.0.as_mut() else { return };
    let (extracted_view, target, depth) = view.into_inner();
    let view_proj = extracted_view
        .clip_from_world
        .unwrap_or_else(|| extracted_view.clip_from_view * extracted_view.world_from_view.to_matrix().inverse());
    // `blackbox_gpu_passes` is Bevy-free and pins its own `glam`, a different crate version from
    // Bevy's `bevy_math::Mat4` even though both are named `glam::Mat4`; round-trip through the array
    // form, like `apply/instances.rs` does the other way for `GlobalTransform`.
    let inverse_view_proj = glam::Mat4::from_cols_array(&view_proj.inverse().to_cols_array());
    let draw = SoftDraw {
        target: target.main_texture_view(),
        depth: &depth.attachment.texture.default_view,
        globals: &inner.bindings.globals_bind_group,
        inverse_view_proj,
    };
    let encoder = ctx.command_encoder();
    inner.effects.draw_soft(device.wgpu_device(), &queue, encoder, &draw);
}
