//! A fullscreen filter: one WGSL module of fragment entry points that read one or two images and a
//! small parameter block, and write one image. Post effects are built on it.
//!
//! An effect's shader is [`POST_COMMON_WGSL`] (the bindings and the vertex stage `vs_main`) followed by
//! the effect's own fragment stages; [`filter_source`] joins them. The bindings are group 0:
//! `src` (0) and `src2` (1) are filterable 2D float textures, `samp` (2) a clamping bilinear sampler and
//! `params` (3) a [`Params`] uniform.

use std::collections::HashMap;

/// The shared half of every filter shader: the `Params` block, the bindings and the vertex stage that
/// draws one oversized triangle.
pub const POST_COMMON_WGSL: &str = include_str!("shaders/post_common.wgsl");

/// The complete shader of an effect whose own fragment stages are `effect`.
pub fn filter_source(effect: &str) -> String {
    format!("{POST_COMMON_WGSL}{effect}")
}

/// The parameter block every effect shader declares as `Params { a: vec4, b: vec4 }`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    pub a: [f32; 4],
    pub b: [f32; 4],
}

/// How a draw combines with what the target already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blend {
    /// Overwrite every texel.
    Replace,
    /// Add to the target's contents (the target is loaded, not cleared).
    Add,
}

/// One draw of a filter.
pub struct Draw<'a> {
    pub entry: &'static str,
    /// Bound as `src`.
    pub src: &'a wgpu::TextureView,
    /// Bound as `src2`; pass the same view as `src` when the effect has one input.
    pub src2: &'a wgpu::TextureView,
    pub target: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub blend: Blend,
}

/// A pipeline cache and a parameter buffer for one filter shader.
pub struct Filter {
    label: &'static str,
    shader: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    pipelines: HashMap<(&'static str, wgpu::TextureFormat, Blend), wgpu::RenderPipeline>,
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

impl Filter {
    /// A filter from the complete shader `source` (see [`filter_source`]).
    pub fn new(device: &wgpu::Device, label: &'static str, source: &str) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { label, shader, layout, pipeline_layout, sampler, params, pipelines: HashMap::new() }
    }

    /// Upload the parameters every draw of this frame's encode reads. Queue writes land before the
    /// submitted commands run, so one block serves all the draws of an encode.
    pub fn set_params(&self, queue: &wgpu::Queue, params: &Params) {
        queue.write_buffer(&self.params, 0, bytemuck::bytes_of(params));
    }

    fn pipeline(&mut self, device: &wgpu::Device, draw: &Draw<'_>) -> &wgpu::RenderPipeline {
        let key = (draw.entry, draw.format, draw.blend);
        self.pipelines.entry(key).or_insert_with(|| {
            let blend = match draw.blend {
                Blend::Replace => None,
                Blend::Add => Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(self.label),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some(draw.entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: draw.format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    /// Record one fullscreen draw into its own render pass.
    pub fn draw(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, draw: &Draw<'_>) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(self.label),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(draw.src) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(draw.src2) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: self.params.as_entire_binding() },
            ],
        });
        let load = match draw.blend {
            Blend::Replace => wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            Blend::Add => wgpu::LoadOp::Load,
        };
        let pipeline = self.pipeline(device, draw);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(draw.entry),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: draw.target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
