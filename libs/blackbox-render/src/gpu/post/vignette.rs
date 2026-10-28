//! Independent screen-edge darkening; never encoded into the UI mask.
use super::filter::{Blend, Draw, Filter};
use super::{PassContext, PassIo, PostPass};
use blackbox_gfx::ScreenGrade;

pub(super) struct VignettePass {
    filter: Filter,
    intensity: f32,
}

impl VignettePass {
    pub(super) fn new(device: &wgpu::Device, intensity: f32, cache: Option<&wgpu::PipelineCache>) -> Self {
        Self {
            filter: Filter::new(device, "vignette", &super::grade::source()).with_pipeline_cache(cache.cloned()),
            intensity,
        }
    }
}

impl PostPass for VignettePass {
    fn name(&self) -> &'static str {
        "vignette"
    }
    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let intensity = match ctx.frame_generation || ctx.secondary_view {
            true => 0.0,
            false => self.intensity,
        };
        let grade = ScreenGrade { edge: [1.0 - intensity; 3], inner: 0.3, ..Default::default() };
        self.filter.set_params(ctx.queue, &super::grade::params(&grade));
        self.filter.draw(
            ctx.device,
            encoder,
            &Draw {
                entry: "fs_main",
                src: io.input,
                src2: io.input,
                target: io.output,
                format: io.output_format,
                blend: Blend::Replace,
            },
        );
    }
}
