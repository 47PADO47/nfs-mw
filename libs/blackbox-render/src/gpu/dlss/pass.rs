//! DLSS Super Resolution/DLAA before bloom, colour grading and the full-resolution UI.
use super::super::post::resolve::Resolve;
use super::super::post::{Extent, PassContext, PassIo, PostPass};
use super::native;

pub(in crate::gpu) struct DlssPass {
    device: wgpu::Device,
    mode: u32,
    output: Option<wgpu::TextureView>,
    copy: Resolve,
    failed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    size: (u32, u32),
}

impl DlssPass {
    pub(in crate::gpu) fn new(
        device: &wgpu::Device,
        quality: blackbox_gfx::UpscaleQuality,
        cache: Option<&wgpu::PipelineCache>,
        failed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            device: device.clone(),
            mode: super::mode(quality),
            output: None,
            copy: Resolve::new(device, cache),
            failed,
            size: (0, 0),
        }
    }
}

impl Drop for DlssPass {
    fn drop(&mut self) {
        native::release(&self.device);
    }
}

impl PostPass for DlssPass {
    fn name(&self) -> &'static str {
        "dlss"
    }
    fn extent(&self) -> Extent {
        Extent::Output
    }
    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let (Some(frame), Some(motion)) = (ctx.temporal, ctx.scene.motion.as_ref()) else {
            return self.copy.encode(ctx, io, encoder);
        };
        if self.failed.load(std::sync::atomic::Ordering::Acquire) {
            return self.copy.encode(ctx, io, encoder);
        }
        if self.size != io.output_size {
            if self.output.is_some() {
                native::release(ctx.device);
            }
            let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("DLSS output"),
                size: wgpu::Extent3d { width: io.output_size.0, height: io.output_size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            self.output = Some(texture.create_view(&Default::default()));
            self.size = io.output_size;
        }
        let result = self.output.as_ref().expect("output created above");
        if let Err(error) = native::evaluate(
            ctx.device,
            ctx.queue,
            encoder,
            [io.input, &ctx.scene.depth_view, &motion.view, result],
            frame,
            self.mode,
        ) {
            log::error!("DLSS disabled for this pass: {error}");
            self.failed.store(true, std::sync::atomic::Ordering::Release);
            return self.copy.encode(ctx, io, encoder);
        }
        self.copy.encode(
            ctx,
            &PassIo {
                input: result,
                input_size: io.output_size,
                output: io.output,
                output_format: io.output_format,
                output_size: io.output_size,
            },
            encoder,
        );
    }
}
