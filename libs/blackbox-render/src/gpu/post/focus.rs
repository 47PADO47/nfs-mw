//! A retained, scene-owned additive mask pass.
use super::filter::{Blend, Draw, Filter, Params, filter_source};
use super::{PassContext, PassIo, PostChain, PostPass};
use blackbox_gfx::{FocusFilter, RgbaImage};

pub(super) fn source() -> String {
    filter_source(include_str!("../../shaders/focus.wgsl"))
}

pub(super) struct FocusPass {
    filter: Filter,
    mask: wgpu::TextureView,
    params: Params,
}

impl FocusPass {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, mask: &RgbaImage) -> Self {
        Self {
            filter: Filter::new(device, "screen mask", &source()),
            mask: super::treatment::gain_texture(device, queue, Some(mask)),
            params: Params::default(),
        }
    }
}

impl PostPass for FocusPass {
    fn name(&self) -> &'static str {
        "screen mask"
    }

    fn set_focus(&mut self, settings: FocusFilter) {
        let s = settings.sanitized();
        self.params = Params { a: [s.vignette, 0.0, 0.0, 0.0], b: [0.0; 4] };
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.filter.set_params(ctx.queue, &self.params);
        self.filter.draw(
            ctx.device,
            encoder,
            &Draw {
                entry: "fs_main",
                src: io.input,
                src2: &self.mask,
                target: io.output,
                format: io.output_format,
                blend: Blend::Replace,
            },
        );
    }
}

impl PostChain {
    pub(in crate::gpu) fn set_focus_mask(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mask: Option<&RgbaImage>,
    ) {
        self.remove("screen mask");
        let mask = mask.filter(|m| {
            m.width > 0 && m.height > 0 && m.rgba.len() as u64 == u64::from(m.width) * u64::from(m.height) * 4
        });
        if let Some(mask) = mask {
            // After the radial blur when that is in the chain, so the mask adds to the blurred frame.
            let index = self.effect_count + usize::from(self.has(super::radial_blur::NAME));
            self.insert(index, Box::new(FocusPass::new(device, queue, mask)));
        }
    }

    /// Apply the scene's screen effects: the radial blur comes into the chain while its strength is above zero,
    /// and every pass takes the settings.
    pub(in crate::gpu) fn set_focus_filter(&mut self, device: &wgpu::Device, settings: FocusFilter) {
        self.set_radial_blur(device, settings.sanitized().radial_blur > 0.0);
        for pass in &mut self.passes {
            pass.set_focus(settings);
        }
    }
}
