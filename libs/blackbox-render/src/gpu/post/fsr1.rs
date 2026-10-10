//! The FSR 1 passes (`blackbox-gpu-passes`) as links of the post chain. Both write the output size
//! ([`Extent::Output`]), so they sit last in the chain, right before the resolve pass, which then copies
//! the sharpened image exactly.

use blackbox_gpu_passes::{Fsr1Io, Fsr1Pass};

use super::{Extent, PassContext, PassIo, PostPass};

pub(in crate::gpu) use blackbox_gpu_passes::{FSR1_EASU as EASU, FSR1_RCAS as RCAS, RcasScale};

struct Fsr1(Fsr1Pass);

/// The upscale pass, then the sharpening pass when `sharpen` is set.
pub(in crate::gpu) fn passes(device: &wgpu::Device, rcas: &RcasScale, sharpen: bool) -> Vec<Box<dyn PostPass>> {
    let passes = blackbox_gpu_passes::fsr1_passes(device, rcas, sharpen);
    passes.into_iter().map(|pass| Box::new(Fsr1(pass)) as Box<dyn PostPass>).collect()
}

impl PostPass for Fsr1 {
    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn extent(&self) -> Extent {
        Extent::Output
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        let io = Fsr1Io {
            input: io.input,
            input_size: io.input_size,
            output: io.output,
            output_format: io.output_format,
            output_size: io.output_size,
        };
        self.0.encode(ctx.device, ctx.queue, encoder, &io);
    }
}
