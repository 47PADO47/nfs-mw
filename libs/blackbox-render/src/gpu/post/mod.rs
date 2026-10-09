//! The post-process chain: an ordered list of fullscreen passes between the HDR scene image and
//! the output image. The UI is drawn after the chain, so it is never post-processed or upscaled.
//!
//! The chain always ends with the built-in [`resolve`] pass, which writes the output. Later passes
//! (bloom, tonemapping, upscaling) are inserted before it with [`PostChain::insert`].

mod plan;
mod resolve;
#[cfg(test)]
mod tests;

use super::targets::FrameTargets;
use plan::{Source, Target};

/// How big the image a pass writes is, when it does not write the final output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Extent {
    /// The internal render size (the scene image's size).
    Render,
    /// The output size (an upscaling pass writes this).
    #[allow(dead_code, reason = "for the upscaling pass of a later layer")]
    Output,
}

/// Everything a pass may read besides its own input.
pub(super) struct PassContext<'a> {
    pub device: &'a wgpu::Device,
    #[allow(dead_code, reason = "for passes that upload parameters")]
    pub queue: &'a wgpu::Queue,
    /// The scene's HDR colour and its depth buffer (reverse-Z, bindable as a sampled depth
    /// texture), both at the render size. The same for every pass of the frame, whatever its input.
    pub scene: &'a FrameTargets,
    pub output_size: (u32, u32),
}

/// The image a pass reads and the one it writes.
pub(super) struct PassIo<'a> {
    pub input: &'a wgpu::TextureView,
    pub input_size: (u32, u32),
    pub output: &'a wgpu::TextureView,
    pub output_format: wgpu::TextureFormat,
    pub output_size: (u32, u32),
}

pub(super) trait PostPass {
    fn name(&self) -> &'static str;

    /// The size of the image this pass writes when it is not the last one.
    fn extent(&self) -> Extent {
        Extent::Render
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder);
}

/// A ping-pong image between two passes.
struct Scratch {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
    format: wgpu::TextureFormat,
}

impl Scratch {
    fn new(device: &wgpu::Device, size: (u32, u32), format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("post-process scratch"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { _texture: texture, view, size, format }
    }
}

pub(super) struct PostChain {
    /// Never empty: the last pass is the resolve pass.
    passes: Vec<Box<dyn PostPass>>,
    scratch: [Option<Scratch>; 2],
}

impl PostChain {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self { passes: vec![Box::new(resolve::Resolve::new(device))], scratch: [None, None] }
    }

    /// Insert `pass` so it runs at `index` among the passes (0 = first). Past-the-end indices put
    /// it right before the resolve pass, which always stays last.
    #[allow(dead_code, reason = "the entry point for the post-process passes of later layers")]
    pub(super) fn insert(&mut self, index: usize, pass: Box<dyn PostPass>) {
        let index = index.min(self.passes.len() - 1);
        self.passes.insert(index, pass);
    }

    /// The pass names in execution order.
    #[allow(dead_code, reason = "diagnostics for later layers and tests")]
    pub(super) fn names(&self) -> Vec<&'static str> {
        self.passes.iter().map(|p| p.name()).collect()
    }

    /// Record every pass. The last one writes `output`.
    pub(super) fn encode(
        &mut self,
        ctx: &PassContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        output: (&wgpu::TextureView, wgpu::TextureFormat),
    ) {
        let Self { passes, scratch } = self;
        let steps = plan::plan(passes.len());
        let scene = ctx.scene;
        for (pass, step) in passes.iter_mut().zip(steps) {
            if let Target::Scratch(slot) = step.target {
                let size = match pass.extent() {
                    Extent::Render => scene.size,
                    Extent::Output => ctx.output_size,
                };
                let stale = scratch[slot].as_ref().is_none_or(|s| s.size != size || s.format != scene.color_format);
                if stale {
                    scratch[slot] = Some(Scratch::new(ctx.device, size, scene.color_format));
                }
            }
            let (input, input_size) = match step.source {
                Source::Scene => (&scene.color_view, scene.size),
                Source::Scratch(slot) => scratch[slot].as_ref().map(|s| (&s.view, s.size)).expect("written earlier"),
            };
            let (view, format, size) = match step.target {
                Target::Output => (output.0, output.1, ctx.output_size),
                Target::Scratch(slot) => {
                    scratch[slot].as_ref().map(|s| (&s.view, s.format, s.size)).expect("created above")
                }
            };
            let io = PassIo { input, input_size, output: view, output_format: format, output_size: size };
            pass.encode(ctx, &io, encoder);
        }
    }
}
