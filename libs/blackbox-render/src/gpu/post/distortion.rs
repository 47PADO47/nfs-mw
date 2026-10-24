//! Sparse displacement quads on the completed scene, before display mapping and UI.
use super::{Extent, PassContext, PassIo, PostChain, PostPass, resolve::Resolve};
use blackbox_gfx::DistortionRegion;
use blackbox_gpu_passes::BindCache;
use std::collections::HashMap;

pub(super) const NAME: &str = "screen displacement";
struct DistortionPass {
    copy: Resolve,
    view: wgpu::TextureView,
    vertices: Vec<[f32; 10]>,
    buffer: wgpu::Buffer,
    capacity: usize,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    shader: wgpu::ShaderModule,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    groups: BindCache<wgpu::TextureView, wgpu::BindGroup>,
    cache: Option<wgpu::PipelineCache>,
}
fn buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(NAME),
        size: (capacity * 40) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
impl DistortionPass {
    fn new(device: &wgpu::Device, view: &wgpu::TextureView, cache: Option<&wgpu::PipelineCache>) -> Self {
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(NAME),
            entries: &[
                texture(0),
                texture(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(NAME),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(NAME),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(NAME),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/screen_distortion.wgsl").into()),
        });
        Self {
            copy: Resolve::new(device, cache),
            view: view.clone(),
            vertices: Vec::new(),
            buffer: buffer(device, 128),
            capacity: 128,
            layout,
            pipeline_layout,
            sampler,
            shader,
            pipelines: HashMap::new(),
            groups: BindCache::default(),
            cache: cache.cloned(),
        }
    }
}
impl PostPass for DistortionPass {
    fn name(&self) -> &'static str {
        NAME
    }
    fn extent(&self) -> Extent {
        Extent::Output
    }
    fn set_distortion(&mut self, view: &wgpu::TextureView, regions: &[DistortionRegion]) {
        self.view = view.clone();
        self.vertices.clear();
        for region in regions {
            let [x, y, w, h] = region.rect;
            if ![x, y, w, h, region.opacity, region.brightness].iter().all(|v| v.is_finite())
                || w <= 0.0
                || h <= 0.0
                || !region.sample_transform.iter().all(|v| v.is_finite())
                || !region.uv_rect.iter().all(|v| v.is_finite())
            {
                continue;
            }
            let [a, b, c, d] = region.sample_transform;
            let [ux, uy, uw, uh] = region.uv_rect;
            for [u, v] in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
                self.vertices.push([
                    (x + w * u) * 2.0 - 1.0,
                    1.0 - (y + h * v) * 2.0,
                    ux + uw * u,
                    uy + uh * v,
                    a,
                    b,
                    c,
                    d,
                    region.opacity,
                    region.brightness,
                ]);
            }
        }
    }
    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.copy.encode(ctx, io, encoder);
        if self.vertices.is_empty() {
            return;
        }
        if self.vertices.len() > self.capacity {
            self.capacity = self.vertices.len().next_power_of_two();
            self.buffer = buffer(ctx.device, self.capacity);
        }
        ctx.queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.vertices));
        let pipeline = self.pipelines.entry(io.output_format).or_insert_with(|| {
            ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(NAME),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 40,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x4,3=>Float32x2],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: io.output_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: self.cache.as_ref(),
            })
        });
        let group = self.groups.get_or_insert_with(io.input, &self.view, || {
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(NAME),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(io.input) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(NAME),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: io.output,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        pass.draw(0..self.vertices.len() as u32, 0..1);
    }
}
impl PostChain {
    pub(in crate::gpu) fn set_distortion(
        &mut self,
        device: &wgpu::Device,
        view: Option<&wgpu::TextureView>,
        regions: &[DistortionRegion],
    ) {
        let Some(view) = view.filter(|_| !regions.is_empty()) else {
            self.remove(NAME);
            return;
        };
        if !self.has(NAME) {
            let _trace = blackbox_trace::scope("Renderer CPU: lens distortion pass created");
            self.insert(usize::MAX, Box::new(DistortionPass::new(device, view, self.cache.as_ref())));
        }
        for pass in &mut self.passes {
            if pass.name() == NAME {
                pass.set_distortion(view, regions);
            }
        }
    }
}
