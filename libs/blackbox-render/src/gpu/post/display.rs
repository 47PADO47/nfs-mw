//! Final display mapping follows all artistic effects and spatial upscaling.
use super::filter::{Blend, Draw, Filter, Params};
use super::{Extent, PassContext, PassIo, PostChain, PostPass};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::gpu) struct Mapping {
    /// Peak relative to reference white. SDR previews use 1.0.
    pub peak: f32,
    pub white: f32,
    pub linear: bool,
}
pub(super) fn source() -> String {
    [include_str!("../../shaders/psychov/core.wgsl"),include_str!("../../shaders/psychov/finish.wgsl"),
        "\n@vertex fn vs_main(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32> { let p=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0)); return vec4(p[i],0.0,1.0); }"].concat()
}
struct DisplayPass {
    filter: Filter,
    mapping: Mapping,
}
impl DisplayPass {
    fn new(device: &wgpu::Device, mapping: Mapping, cache: Option<&wgpu::PipelineCache>) -> Self {
        let filter = Filter::new(device, "PsychoV display", &source()).with_pipeline_cache(cache.cloned());
        Self { filter, mapping }
    }
    fn draw(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder, diagnostic: bool) {
        let m = self.mapping;
        self.filter.set_params(
            ctx.queue,
            &Params {
                a: [m.peak, m.white, u8::from(m.linear) as f32, 0.0],
                b: [u8::from(diagnostic) as f32, io.output_size.0 as f32, io.output_size.1 as f32, 0.0],
            },
        );
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
impl PostPass for DisplayPass {
    fn name(&self) -> &'static str {
        "hdr-display"
    }
    fn extent(&self) -> Extent {
        Extent::Output
    }
    fn set_display(&mut self, mapping: Option<Mapping>) {
        if let Some(v) = mapping {
            self.mapping = v;
        }
    }
    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.draw(ctx, io, encoder, false);
    }
    fn encode_diagnostic(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.draw(ctx, io, encoder, true);
    }
}
impl PostChain {
    pub(in crate::gpu) fn set_display(&mut self, device: &wgpu::Device, mapping: Option<Mapping>) {
        if self.display == mapping {
            return;
        }
        let changed = self.display.is_some() != mapping.is_some();
        self.display = mapping;
        if changed {
            self.remove("hdr-display");
            if let Some(v) = mapping {
                // The display pass writes the output itself and stays last; the resolve copy is not in `passes`.
                self.passes.push(Box::new(DisplayPass::new(device, v, self.cache.as_ref())));
            }
            self.rebuild_effects(device);
        }
        for pass in &mut self.passes {
            pass.set_display(mapping);
        }
    }
}
