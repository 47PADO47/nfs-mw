//! The speed and pursuit radial blur as a post pass. It sits after the post effects and before the speedbreaker
//! mask, so it smears the finished frame before the HUD. It is in the chain only while its strength is above zero:
//! at zero it costs nothing.

use super::{PassContext, PassIo, PostChain, PostPass};
use blackbox_gfx::FocusFilter;
use blackbox_gpu_passes::{RadialBlur, RadialBlurIo, RadialBlurSettings};

/// The name the chain knows the pass by.
pub(super) const NAME: &str = "radial blur";

pub(super) struct RadialBlurPass {
    blur: RadialBlur,
}

impl RadialBlurPass {
    fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        Self { blur: RadialBlur::new(device).with_pipeline_cache(cache.cloned()) }
    }
}

impl PostPass for RadialBlurPass {
    fn name(&self) -> &'static str {
        NAME
    }

    fn set_focus(&mut self, settings: FocusFilter) {
        let s = settings.sanitized();
        let current = self.blur.settings();
        self.blur.set(RadialBlurSettings { strength: s.radial_blur, centre: s.radial_centre, ..current });
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let io = RadialBlurIo { input: io.input, output: io.output, output_format: io.output_format };
        self.blur.encode(ctx.device, ctx.queue, encoder, &io);
    }
}

impl PostChain {
    /// Keep the radial blur in the chain exactly while `on`. It goes right after the post effects, so the
    /// speedbreaker mask (which follows it) adds to the blurred image.
    pub(in crate::gpu) fn set_radial_blur(&mut self, device: &wgpu::Device, on: bool) {
        if !on {
            self.remove(NAME);
            return;
        }
        if self.has(NAME) {
            return;
        }
        self.insert(self.effect_count, Box::new(RadialBlurPass::new(device, self.cache.as_ref())));
    }
}
