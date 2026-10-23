// Ported from the smaa crate 0.21.0 (src/lib.rs, the Pipelines type): the three fullscreen passes, each a vertex
// and a fragment entry point of one WGSL module, with the same targets and blend state as the reference.
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

use crate::bindings::BindGroupLayouts;
use crate::shaders::{BLEND_WEIGHT_WGSL, EDGE_DETECT_WGSL, NEIGHBORHOOD_WGSL};

pub(crate) struct Pipelines {
    pub(crate) edge_detect: wgpu::RenderPipeline,
    pub(crate) blend_weight: wgpu::RenderPipeline,
    pub(crate) neighborhood_blending: wgpu::RenderPipeline,
}

impl Pipelines {
    /// Build the three pipelines. `format` is the output format of the last pass; the intermediate targets use
    /// the formats the reference uses (RG8 edges, RGBA8 weights). Every pipeline is built with `cache`.
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        layouts: &BindGroupLayouts,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let edge_module = shader_module(device, "smaa.shader.edge_detect", EDGE_DETECT_WGSL);
        let blend_module = shader_module(device, "smaa.shader.blend_weight", BLEND_WEIGHT_WGSL);
        let neighborhood_module = shader_module(device, "smaa.shader.neighborhood_blending", NEIGHBORHOOD_WGSL);
        Self {
            edge_detect: pass_pipeline(
                device,
                "smaa.pipeline.edge_detect",
                &layouts.edge_detect,
                &edge_module,
                wgpu::TextureFormat::Rg8Unorm,
                Some(StencilMask::Write),
                cache,
            ),
            blend_weight: pass_pipeline(
                device,
                "smaa.pipeline.blend_weight",
                &layouts.blend_weight,
                &blend_module,
                wgpu::TextureFormat::Rgba8Unorm,
                Some(StencilMask::Test),
                cache,
            ),
            neighborhood_blending: pass_pipeline(
                device,
                "smaa.pipeline.neighborhood_blending",
                &layouts.neighborhood_blending,
                &neighborhood_module,
                format,
                None,
                cache,
            ),
        }
    }
}

/// The stencil mask the edge pass writes and the blend-weight pass tests (reference value 1).
#[derive(Clone, Copy)]
enum StencilMask {
    /// Replace the stencil with the reference where a fragment survives (the edge pass discards pixels without edges).
    Write,
    /// Draw only where the stencil equals the reference.
    Test,
}

impl StencilMask {
    fn state(self) -> wgpu::DepthStencilState {
        let face = match self {
            Self::Write => wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Always,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Replace,
            },
            Self::Test => wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Equal,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            },
        };
        let write_mask = match self {
            Self::Write => 0xff,
            Self::Test => 0,
        };
        wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Stencil8,
            depth_write_enabled: None,
            depth_compare: None,
            stencil: wgpu::StencilState { front: face, back: face, read_mask: 0xff, write_mask },
            bias: Default::default(),
        }
    }
}

fn shader_module(device: &wgpu::Device, label: &str, source: &'static str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

/// A fullscreen-triangle pipeline: `vs_main` and `fs_main` of `module`, one colour target, no blending.
fn pass_pipeline(
    device: &wgpu::Device,
    label: &str,
    bind_group_layout: &wgpu::BindGroupLayout,
    module: &wgpu::ShaderModule,
    target: wgpu::TextureFormat,
    mask: Option<StencilMask>,
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent::REPLACE,
                    alpha: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: Default::default(),
        depth_stencil: mask.map(StencilMask::state),
        multisample: Default::default(),
        multiview_mask: None,
        cache,
    })
}
