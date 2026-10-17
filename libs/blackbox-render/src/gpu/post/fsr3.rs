//! The FSR 3 temporal upscaler (`libs/fsr3-wgpu`) as the first link of the post chain.
//!
//! It reads the scene's jittered HDR colour, its depth and its motion vectors, all at the render size, and
//! writes the upscaled image to a storage texture of its own at the output size. A render attachment cannot be
//! a storage texture, so the result is then copied into the chain's image for the next pass (bloom, tone
//! mapping), which is the same copy the plain resolve pass makes.

use fsr3_wgpu::{Fsr3Config, Fsr3Context, Fsr3Error, Fsr3Inputs, Fsr3Outputs};
use glam::UVec2;

use super::resolve::Resolve;
use super::{Extent, PassContext, PassIo, PostPass};
use crate::gpu::temporal::TemporalFrame;

/// The name the chain knows the pass by.
pub(in crate::gpu) const NAME: &str = "fsr3";

pub(in crate::gpu) struct Fsr3Pass {
    context: Fsr3Context,
    /// The upscaler's result, with the output size it was made for.
    output: Option<(UVec2, wgpu::TextureView)>,
    /// Copies the result into the chain, and is the fallback if an upscale cannot be recorded.
    copy: Resolve,
}

impl Fsr3Pass {
    /// The upscaler's pipelines are built here, so choosing FSR 3 pays for them once, not on the first frame.
    pub(in crate::gpu) fn new(device: &wgpu::Device) -> Result<Self, Fsr3Error> {
        let context = Fsr3Context::new(device, Fsr3Config::new())?;
        Ok(Self { context, output: None, copy: Resolve::new(device) })
    }

    /// The storage texture the upscaler writes, made again when the output size changes.
    fn target(&mut self, device: &wgpu::Device, size: UVec2) -> &wgpu::TextureView {
        let stale = !matches!(&self.output, Some((made, _)) if *made == size);
        if stale {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("fsr3 output"),
                size: wgpu::Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.context.config().output_format,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.output = Some((size, texture.create_view(&wgpu::TextureViewDescriptor::default())));
        }
        &self.output.as_ref().expect("made above").1
    }

    /// Record the upscale of `io.input` into the pass's own texture.
    fn upscale(
        &mut self,
        ctx: &PassContext<'_>,
        io: &PassIo<'_>,
        frame: &TemporalFrame,
        motion: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), Fsr3Error> {
        let render_size = UVec2::new(io.input_size.0, io.input_size.1);
        let output_size = UVec2::new(io.output_size.0, io.output_size.1);
        let output = self.target(ctx.device, output_size).clone();
        let inputs = Fsr3Inputs {
            color: io.input,
            depth: &ctx.scene.depth_view,
            motion_vectors: motion,
            exposure: None,
            reactive: None,
            transparency_and_composition: None,
            render_size,
            jitter: frame.jitter,
            motion_vector_scale: fsr3_wgpu::motion_vector_scale_ndc(render_size),
            delta_time: frame.delta_time,
            pre_exposure: 0.0,
            sharpness: frame.sharpness,
            camera_near: frame.near,
            // The projection is reverse-Z with no far plane, so the far value is ignored.
            camera_far: f32::INFINITY,
            camera_fov_y: frame.fov_y,
            // The world is in metres.
            view_space_to_meters: 1.0,
            reset: frame.reset,
        };
        let outputs = Fsr3Outputs { output: &output, size: output_size };
        self.context.dispatch(ctx.device, ctx.queue, encoder, &inputs, &outputs)
    }
}

impl PostPass for Fsr3Pass {
    fn name(&self) -> &'static str {
        NAME
    }

    fn extent(&self) -> Extent {
        Extent::Output
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let motion = ctx.scene.motion.as_ref();
        let (Some(frame), Some(motion)) = (ctx.temporal, motion) else {
            return self.copy.encode(ctx, io, encoder);
        };
        if let Err(error) = self.upscale(ctx, io, frame, &motion.view, encoder) {
            log::error!("fsr3: the upscale could not be recorded: {error}");
            return self.copy.encode(ctx, io, encoder);
        }
        let Some((size, result)) = &self.output else { return };
        let io = PassIo {
            input: result,
            input_size: (size.x, size.y),
            output: io.output,
            output_format: io.output_format,
            output_size: io.output_size,
        };
        self.copy.encode(ctx, &io, encoder);
    }
}
