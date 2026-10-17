// Ported from the smaa crate 0.21.0 (src/lib.rs, BindGroupLayouts and BindGroups):
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
//
// The bindings of the shaders in src/shaders: group 0, binding 0 the linear sampler, binding 1 the metrics
// uniform (visible to the vertex stage too), bindings 2.. the textures, in the order each pass declares them.

use crate::resources::Resources;
use crate::targets::Targets;

pub(crate) struct BindGroupLayouts {
    pub(crate) edge_detect: wgpu::BindGroupLayout,
    pub(crate) blend_weight: wgpu::BindGroupLayout,
    pub(crate) neighborhood_blending: wgpu::BindGroupLayout,
}

pub(crate) struct BindGroups {
    pub(crate) edge_detect: wgpu::BindGroup,
    pub(crate) blend_weight: wgpu::BindGroup,
    pub(crate) neighborhood_blending: wgpu::BindGroup,
}

impl BindGroupLayouts {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self {
            // Edges: the colour input.
            edge_detect: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("smaa.bind_group_layout.edge_detect"),
                entries: &[sampler(0), metrics(1), texture(2)],
            }),
            // Blend weights: edges, area and search tables.
            blend_weight: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("smaa.bind_group_layout.blend_weight"),
                entries: &[sampler(0), metrics(1), texture(2), texture(3), texture(4)],
            }),
            // Neighbourhood blending: the colour input and the blend weights.
            neighborhood_blending: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("smaa.bind_group_layout.neighborhood_blending"),
                entries: &[sampler(0), metrics(1), texture(2), texture(3)],
            }),
        }
    }
}

fn sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn metrics(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

impl BindGroups {
    pub(crate) fn new(
        device: &wgpu::Device,
        layouts: &BindGroupLayouts,
        resources: &Resources,
        targets: &Targets,
    ) -> Self {
        let uniforms =
            wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &targets.rt_uniforms, offset: 0, size: None });
        let sampler_res = wgpu::BindingResource::Sampler(&resources.sampler);
        Self {
            edge_detect: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("smaa.bind_group.edge_detect"),
                layout: &layouts.edge_detect,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: sampler_res.clone() },
                    wgpu::BindGroupEntry { binding: 1, resource: uniforms.clone() },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&targets.color) },
                ],
            }),
            blend_weight: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("smaa.bind_group.blend_weight"),
                layout: &layouts.blend_weight,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: sampler_res.clone() },
                    wgpu::BindGroupEntry { binding: 1, resource: uniforms.clone() },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&targets.edges) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&resources.area) },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&resources.search),
                    },
                ],
            }),
            neighborhood_blending: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("smaa.bind_group.neighborhood_blending"),
                layout: &layouts.neighborhood_blending,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: sampler_res },
                    wgpu::BindGroupEntry { binding: 1, resource: uniforms },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&targets.color) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&targets.blend) },
                ],
            }),
        }
    }
}
