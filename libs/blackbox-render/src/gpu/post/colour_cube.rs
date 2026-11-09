//! Colour grading through 3D look-up tables as a post pass (`blackbox_gpu_passes::ColourCubePass`, spec
//! `docs/specs/colour-grading.md`). It sits right after the post effects (after tone mapping and anti-aliasing), so the
//! cubes see the display-range colours they were authored for, and before the scene's own screen effects (radial blur,
//! screen grade) and the upscalers.
//!
//! The pass is put into the chain the first time a grade arrives and stays there; while no grade is in force it is inactive
//! (left out of every frame's plan), so a scene that does not grade pays nothing.

use super::{PassContext, PassIo, PostChain, PostPass};
use blackbox_gfx::ColourGrade;
use blackbox_gpu_passes::{ColourCubeIo, ColourCubePass};

/// The name the chain knows the pass by.
pub(super) const NAME: &str = "colour cube";

/// What decides whether the drawn image changes: the cubes, the blend and the strength, the last two to 1/256.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GradeKey {
    from: u64,
    to: Option<u64>,
    mix: u8,
    strength: u8,
}

impl GradeKey {
    /// The key of `grade` (sanitised), `None` for a grade that draws nothing.
    pub(super) fn of(grade: Option<&ColourGrade>) -> Option<Self> {
        let g = grade?.sanitized();
        if g.strength <= 0.0 {
            return None;
        }
        let byte = |v: f32| (v * 255.0).round() as u8;
        Some(Self {
            from: g.from.id(),
            to: g.to.as_ref().map(|c| c.id()),
            mix: byte(g.mix),
            strength: byte(g.strength),
        })
    }
}

pub(super) struct ColourCubePostPass {
    pass: ColourCubePass,
}

impl ColourCubePostPass {
    fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        Self { pass: ColourCubePass::new(device).with_pipeline_cache(cache.cloned()) }
    }
}

impl PostPass for ColourCubePostPass {
    fn name(&self) -> &'static str {
        NAME
    }

    fn active(&self) -> bool {
        self.pass.is_active()
    }

    fn set_colour_grade(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, grade: Option<&ColourGrade>) {
        self.pass.set(device, queue, grade);
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let io = ColourCubeIo { input: io.input, output: io.output, output_format: io.output_format };
        self.pass.encode(ctx.device, ctx.queue, encoder, &io);
    }
}

impl PostChain {
    /// Grade with `grade` from the next frame on, or stop grading with `None` or a grade that draws nothing. Returns what
    /// changed: `None` when the picture is the same as before, else whether the chain gained or lost the pass's work (the
    /// scene's targets then change, see `Renderer::refresh_targets`).
    pub(in crate::gpu) fn set_colour_grade(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        grade: Option<&ColourGrade>,
    ) -> Option<bool> {
        let key = GradeKey::of(grade);
        if key == self.grade_key {
            return None;
        }
        let was_on = self.grade_key.is_some();
        self.grade_key = key;
        if key.is_some() && !self.has(NAME) {
            self.insert(self.effect_count, Box::new(ColourCubePostPass::new(device, self.cache.as_ref())));
        }
        for pass in &mut self.passes {
            pass.set_colour_grade(device, queue, grade);
        }
        Some(was_on != key.is_some())
    }
}
