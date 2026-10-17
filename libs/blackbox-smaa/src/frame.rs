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

use crate::target::SmaaTarget;

/// A frame of an SMAA target. It derefs to the view the scene renders into. [`SmaaFrame::resolve`], or dropping
/// the frame, encodes the three SMAA passes (edges, blend weights, neighbourhood blending) into the output view and
/// submits them.
pub struct SmaaFrame<'a> {
    pub(crate) target: &'a mut SmaaTarget,
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) output_view: &'a wgpu::TextureView,
}

impl SmaaFrame<'_> {
    /// Run the three passes now. Same as dropping the frame.
    pub fn resolve(self) {
        drop(self);
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
        let Some(inner) = self.target.inner.as_ref() else {
            return;
        };
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("smaa.command_encoder") });
        encode_pass(
            &mut encoder,
            "smaa.render_pass.edge_detect",
            &inner.targets.edges,
            &inner.pipelines.edge_detect,
            &inner.bind_groups.edge_detect,
        );
        encode_pass(
            &mut encoder,
            "smaa.render_pass.blend_weight",
            &inner.targets.blend,
            &inner.pipelines.blend_weight,
            &inner.bind_groups.blend_weight,
        );
        encode_pass(
            &mut encoder,
            "smaa.render_pass.neighborhood_blending",
            self.output_view,
            &inner.pipelines.neighborhood_blending,
            &inner.bind_groups.neighborhood_blending,
        );
        self.queue.submit(Some(encoder.finish()));
    }
}

/// One fullscreen triangle into `view`, cleared to black first (the reference clears every intermediate).
fn encode_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}
