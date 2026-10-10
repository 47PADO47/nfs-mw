//! The FSR 1 pass, run in place of Bevy's own bilinear `upscaling` system when the effective upscaler is
//! [`Upscaler::Fsr1`] and the render scale is below 1 ([`fsr1_active`]).
//!
//! Bevy's `upscaling` (`bevy_core_pipeline::upscaling::upscaling`) has no run condition: it always blits
//! the render-size main texture to the output-size texture, whatever the camera's resolution override is.
//! There is no public hook to suppress just that one system from here, so this system runs
//! `.after(upscaling)` and, only when FSR 1 is active, overwrites what it just wrote with EASU (and RCAS,
//! when sharpening) instead. The final image is correct either way; when FSR 1 is on, one extra
//! full-resolution bilinear blit runs and is thrown away. Documented as a known inefficiency in
//! `docs/bevy-backend.md` ("PR 10") rather than patched around, since cleanly removing it needs changing
//! how `UpscalingPlugin` registers its own system, which it does not expose a hook for.

use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Res, ResMut};
use bevy_render::camera::ExtractedCamera;
use bevy_render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy_render::view::ViewTarget;
use blackbox_gfx::{fsr1_active, rcas_stops};
use blackbox_gpu_passes::{Fsr1Io, Fsr1Pass, RcasScale, fsr1_passes};

use crate::ops::BlackboxBridge;

/// The pipelines and the scratch texture EASU's output needs when RCAS also runs, created once the
/// device exists and rebuilt only when sharpening turns on or off (the pass count changes).
#[derive(Resource, Default)]
pub struct Fsr1Render {
    rcas: RcasScale,
    inner: Option<Inner>,
}

struct Inner {
    /// One pass (EASU) or two (EASU, RCAS), from [`fsr1_passes`].
    passes: Vec<Fsr1Pass>,
    /// The output-size texture EASU writes to when RCAS reads it back; unused with EASU alone.
    scratch: Option<Scratch>,
}

struct Scratch {
    /// A `TextureView` keeps its source texture alive on its own (wgpu reference-counts it), so no
    /// `wgpu::Texture` needs to be kept here too.
    view: wgpu::TextureView,
    size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl Fsr1Render {
    fn get_or_init(&mut self, device: &wgpu::Device, sharpen: bool) -> &mut Inner {
        let stale = !matches!(&self.inner, Some(inner) if inner.passes.len() == usize::from(sharpen) + 1);
        if stale {
            self.inner = Some(Inner { passes: fsr1_passes(device, &self.rcas, sharpen), scratch: None });
        }
        self.inner.as_mut().expect("just set")
    }
}

impl Inner {
    /// The EASU output texture at `size`, recreated only when the size or format changed.
    fn scratch_view(
        &mut self,
        device: &wgpu::Device,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureView {
        if let Some(scratch) = &self.scratch
            && scratch.size == size
            && scratch.format == format
        {
            return scratch.view.clone();
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("blackbox fsr1 easu output"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.scratch = Some(Scratch { view: view.clone(), size, format });
        view
    }
}

/// Draws EASU (and RCAS, when sharpening) from the render-size main texture to the output-size texture,
/// in place of Bevy's own bilinear blit. Does nothing when FSR 1 is not the active upscaler at this render
/// scale: no scratch texture, no pipeline, the same "nothing allocated when off" rule as every other
/// post component here.
pub fn fsr1_pass(
    mut render: ResMut<Fsr1Render>,
    bridge: Res<BlackboxBridge>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    view: ViewQuery<(&ExtractedCamera, &ViewTarget)>,
    mut ctx: RenderContext,
) {
    let (upscaler, render_scale, sharpness) = {
        let shared = bridge.lock();
        (shared.settings.upscaler, shared.settings.render_scale, shared.settings.upscale_sharpness)
    };
    if !fsr1_active(upscaler, render_scale) {
        return;
    }
    let (camera, target) = view.into_inner();
    let Some(out_view) = target.out_texture() else { return };
    let Some(size) = camera.physical_target_size else { return };
    let format = target.out_texture_view_format().unwrap_or_else(|| target.main_texture_format());
    let main_view = target.main_texture_view();
    let out_size = (size.x, size.y);
    // `MainPassResolutionOverride` does not shrink `main_texture` itself (`post::scale`'s doc comment on
    // `apply_resolution_override`): the main pass draws into a `render_size`-pixel corner of an
    // `out_size`-sized texture, not a texture sized exactly to `render_size` the way native's own render
    // target is. EASU needs to know that corner's size explicitly (`Fsr1Io::input_size`) rather than
    // trust `textureDimensions`, which would read the whole oversized texture as if it were all valid
    // content.
    let in_size = blackbox_gfx::scaled_size(out_size, render_scale);

    let stops = rcas_stops(sharpness);
    if let Some(stops) = stops {
        render.rcas.set_stops(stops);
    }
    let sharpen = stops.is_some();
    let device = device.wgpu_device();
    let inner = render.get_or_init(device, sharpen);

    if !sharpen {
        let io = Fsr1Io {
            input: main_view,
            input_size: in_size,
            output: out_view,
            output_format: format,
            output_size: out_size,
        };
        inner.passes[0].encode(device, &queue, ctx.command_encoder(), &io);
        return;
    }
    let scratch = inner.scratch_view(device, out_size, format);
    let easu = Fsr1Io {
        input: main_view,
        input_size: in_size,
        output: &scratch,
        output_format: format,
        output_size: out_size,
    };
    inner.passes[0].encode(device, &queue, ctx.command_encoder(), &easu);
    // RCAS reads EASU's own output, which is written tightly at `out_size` (the scratch texture, not the
    // camera's oversized main texture), so its content size is simply `out_size` itself.
    let rcas = Fsr1Io {
        input: &scratch,
        input_size: out_size,
        output: out_view,
        output_format: format,
        output_size: out_size,
    };
    inner.passes[1].encode(device, &queue, ctx.command_encoder(), &rcas);
}
