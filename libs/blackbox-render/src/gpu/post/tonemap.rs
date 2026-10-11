//! The tone-mapping pass: exposure, then a filmic curve (`shaders/tonemap.wgsl`).

use super::filter::{Blend, Draw, Filter, Params, tonemap_wgsl};
use super::{PassContext, PassIo, PostPass};
use crate::PostSettings;

/// The parameter block for `settings`: `a.x` is the exposure.
pub(super) fn params(settings: &PostSettings) -> Params {
    Params { a: [settings.exposure, 0.0, 0.0, 0.0], b: [0.0; 4] }
}

/// The curve as the shader evaluates it, for one channel (the tests compare the GPU against it).
#[cfg(test)]
pub(super) fn aces(x: f32) -> f32 {
    let x = x.max(0.0);
    ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
}

pub(super) struct TonemapPass {
    filter: Filter,
    params: Params,
}

impl TonemapPass {
    pub(super) fn new(device: &wgpu::Device, settings: &PostSettings) -> Self {
        Self { filter: Filter::new(device, "tonemap", &tonemap_wgsl()), params: params(settings) }
    }
}

impl PostPass for TonemapPass {
    fn name(&self) -> &'static str {
        "tonemap"
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.filter.set_params(ctx.queue, &self.params);
        let draw = Draw {
            entry: "fs_main",
            src: io.input,
            src2: io.input,
            target: io.output,
            format: io.output_format,
            blend: Blend::Replace,
        };
        self.filter.draw(ctx.device, encoder, &draw);
    }
}
