//! Small spatial reconstruction of finite-sun visibility; never reuses frame history.
use super::Groups;
use crate::gpu::surface::targets::Targets;
pub(super) struct Filter {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    float_pipeline: wgpu::RenderPipeline,
    /// Inputs keyed by the depth, normal, ray and ray-surface views.
    groups: Groups<4>,
}
/// The views one filter pass reads.
pub(super) struct Inputs<'a> {
    pub params: &'a wgpu::Buffer,
    pub raw: &'a wgpu::TextureView,
    pub surface: &'a wgpu::TextureView,
    pub output: &'a wgpu::TextureView,
    pub targets: &'a Targets,
}
impl Filter {
    pub fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for binding in 1..=4 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: match binding {
                        1 => wgpu::TextureSampleType::Depth,
                        4 => wgpu::TextureSampleType::Uint,
                        _ => wgpu::TextureSampleType::Float { filterable: false },
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sun spatial inputs"),
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sun spatial filter"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/ray_shadow_filter.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sun spatial filter"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make = |format| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sun spatial filter"),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache,
            })
        };
        Self {
            layout,
            pipeline: make(wgpu::TextureFormat::Rg8Unorm),
            float_pipeline: make(wgpu::TextureFormat::Rgba16Float),
            groups: Groups::default(),
        }
    }
    pub fn draw(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, inputs: Inputs<'_>) {
        let Inputs { params, raw, surface, output, targets } = inputs;
        let layout = &self.layout;
        let group = self.groups.get([&targets.depth, &targets.normals, raw, surface], || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sun spatial inputs"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&targets.depth) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&targets.normals) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(raw) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(surface) },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("RT sun spatial reconstruction"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(match output.texture().format() {
            wgpu::TextureFormat::Rgba16Float => &self.float_pipeline,
            _ => &self.pipeline,
        });
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}
pub(super) fn raw(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    target(device, size, "sun raw visibility and blocker distance", wgpu::TextureFormat::Rgba16Float)
}
/// Per ray cell: the representative receiver's camera-relative position and normal, written by the trace.
pub(super) fn surface(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    target(device, size, "sun ray receiver planes", wgpu::TextureFormat::Rgba32Uint)
}
fn target(device: &wgpu::Device, size: (u32, u32), label: &str, format: wgpu::TextureFormat) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
