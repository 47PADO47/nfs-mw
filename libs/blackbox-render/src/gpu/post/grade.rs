//! The scene grade as a post pass (`FocusFilter::grade`): a darkening, tinted vignette, a colour multiplier and a
//! contrast (`shaders/grade.wgsl`). It sits after the radial blur, like the speedbreaker mask, and is in the chain only
//! while the grade is not neutral, so a scene that asks for none pays nothing.

use super::filter::{Blend, Draw, Filter, Params, filter_source};
use super::{PassContext, PassIo, PostChain, PostPass};
use blackbox_gfx::{FocusFilter, ScreenGrade};

/// The name the chain knows the pass by.
pub(super) const NAME: &str = "screen grade";

pub(super) fn source() -> String {
    filter_source(include_str!("../../shaders/grade.wgsl"))
}

/// The parameter block for `grade`: `a` is the edge colour and the inner distance, `b` the tint and the contrast.
pub(super) fn params(grade: &ScreenGrade) -> Params {
    let g = grade.sanitized();
    Params { a: [g.edge[0], g.edge[1], g.edge[2], g.inner], b: [g.tint[0], g.tint[1], g.tint[2], g.contrast] }
}

pub(super) struct GradePass {
    filter: Filter,
    params: Params,
}

impl GradePass {
    fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        Self {
            filter: Filter::new(device, "screen grade", &source()).with_pipeline_cache(cache.cloned()),
            params: params(&ScreenGrade::default()),
        }
    }
}

impl PostPass for GradePass {
    fn name(&self) -> &'static str {
        NAME
    }

    fn set_focus(&mut self, settings: FocusFilter) {
        self.params = params(&settings.grade);
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

impl PostChain {
    /// Keep the grade in the chain exactly while `on`. It goes after the radial blur when that is in the chain, so the
    /// blur smears the frame first and the grade darkens its edge.
    pub(in crate::gpu) fn set_grade(&mut self, device: &wgpu::Device, on: bool) {
        if !on {
            self.remove(NAME);
            return;
        }
        if self.has(NAME) {
            return;
        }
        let index = self.effect_count + usize::from(self.has(super::radial_blur::NAME));
        self.insert(index, Box::new(GradePass::new(device, self.cache.as_ref())));
    }
}
