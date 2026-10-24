// Ported from the smaa crate 0.21.0 (src/lib.rs, the SmaaFrame type, its Deref and its Drop that runs the passes):
// Copyright (c) 2018 Jonathan Behrens
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::target::{Inner, SmaaTarget};

/// A frame of an SMAA target. It derefs to the view the scene renders into. [`SmaaFrame::encode_into`] records the
/// three SMAA passes (edges, blend weights, neighbourhood blending) into a caller encoder; [`SmaaFrame::resolve`], or
/// dropping the frame, records them into an encoder of its own and submits it.
pub struct SmaaFrame<'a> {
    pub(crate) target: &'a mut SmaaTarget,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) output_view: &'a wgpu::TextureView,
    /// The passes were already recorded through [`SmaaFrame::encode_into`].
    pub(crate) encoded: bool,
}

impl SmaaFrame<'_> {
    /// Run the three passes now. Same as dropping the frame.
    pub fn resolve(self) {
        drop(self);
    }

    /// Record the three passes into `encoder` and consume the frame without submitting anything. The caller submits
    /// `encoder` after the scene commands that rendered into this frame, so the whole frame needs one submit.
    pub fn encode_into(mut self, encoder: &mut wgpu::CommandEncoder) {
        self.encoded = true;
        let Some(inner) = self.target.inner.as_ref() else {
            return;
        };
        encode_passes(encoder, inner, self.output_view);
    }
}

impl std::ops::Deref for SmaaFrame<'_> {
    type Target = wgpu::TextureView;

    fn deref(&self) -> &wgpu::TextureView {
        match self.target.inner.as_ref() {
            None => self.output_view,
            Some(inner) => &inner.targets.color,
        }
    }
}

impl Drop for SmaaFrame<'_> {
    fn drop(&mut self) {
        if self.encoded {
            return;
        }
        let Some(inner) = self.target.inner.as_ref() else {
            return;
        };
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("smaa.command_encoder") });
        encode_passes(&mut encoder, inner, self.output_view);
        self.queue.submit(Some(encoder.finish()));
    }
}

/// The three passes. The edge pass marks the pixels that have an edge in the stencil; the blend-weight pass only
/// runs on those, because every other pixel's weights are zero and the target is cleared to zero. The neighbourhood
/// pass reads weights of neighbouring pixels, so it keeps running on every pixel.
pub(crate) fn encode_passes(encoder: &mut wgpu::CommandEncoder, inner: &Inner, output_view: &wgpu::TextureView) {
    encode_pass(
        encoder,
        "smaa.render_pass.edge_detect",
        &inner.targets.edges,
        wgpu::Color::BLACK,
        Some(StencilUse::WriteMask),
        &inner.targets.stencil,
        &inner.pipelines.edge_detect,
        &inner.bind_groups.edge_detect,
    );
    encode_pass(
        encoder,
        "smaa.render_pass.blend_weight",
        &inner.targets.blend,
        wgpu::Color::TRANSPARENT,
        Some(StencilUse::TestMask),
        &inner.targets.stencil,
        &inner.pipelines.blend_weight,
        &inner.bind_groups.blend_weight,
    );
    encode_pass(
        encoder,
        "smaa.render_pass.neighborhood_blending",
        output_view,
        wgpu::Color::BLACK,
        None,
        &inner.targets.stencil,
        &inner.pipelines.neighborhood_blending,
        &inner.bind_groups.neighborhood_blending,
    );
}

/// How a pass uses the stencil mask.
#[derive(Clone, Copy)]
enum StencilUse {
    /// Clear it, then the pipeline writes 1 where a fragment survives.
    WriteMask,
    /// Read it only; the pipeline draws where it is 1.
    TestMask,
}

/// One fullscreen triangle into `view`, cleared to `clear` first (the reference clears every intermediate; the blend
/// target must clear to zero in alpha too, as the stencil skips pixels).
#[allow(clippy::too_many_arguments)]
fn encode_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
    clear: wgpu::Color,
    stencil: Option<StencilUse>,
    stencil_view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
) {
    let stencil_ops = match stencil {
        Some(StencilUse::WriteMask) => {
            Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0), store: wgpu::StoreOp::Store })
        }
        _ => None,
    };
    let depth_stencil_attachment =
        stencil.map(|_| wgpu::RenderPassDepthStencilAttachment { view: stencil_view, depth_ops: None, stencil_ops });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment,
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    if stencil.is_some() {
        pass.set_stencil_reference(1);
    }
    pass.draw(0..3, 0..1);
}
