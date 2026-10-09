//! The FXAA pass (`shaders/fxaa.wgsl`): one fullscreen draw at the size of its input.

use super::filter::{Blend, Draw, FXAA_WGSL, Filter, Params};
use super::{PassContext, PassIo, PostPass};

/// Edges with less local luma contrast than this share of the brightest neighbour are left alone.
const EDGE_THRESHOLD: f32 = 0.125;
/// ...and so are edges with less contrast than this, whatever the brightness (dark areas).
const EDGE_THRESHOLD_MIN: f32 = 0.0312;
/// How much sub-pixel detail (single bright pixels, thin lines) is blended away.
const SUB_PIXEL: f32 = 0.75;

/// The parameter block: `a` = (edge threshold, edge threshold minimum, sub-pixel blend).
pub(super) fn params() -> Params {
    Params { a: [EDGE_THRESHOLD, EDGE_THRESHOLD_MIN, SUB_PIXEL, 0.0], b: [0.0; 4] }
}

pub(super) struct FxaaPass {
    filter: Filter,
}

impl FxaaPass {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self { filter: Filter::new(device, "fxaa", FXAA_WGSL) }
    }
}

impl PostPass for FxaaPass {
    fn name(&self) -> &'static str {
        "fxaa"
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.filter.set_params(ctx.queue, &params());
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
