use super::Shadows;
use crate::Vertex;
use crate::gpu::{instances::INSTANCE_LAYOUT, pipelines::VERTEX_ATTRIBUTES};

impl Shadows {
    pub(super) fn new(device: &wgpu::Device, texture: &wgpu::BindGroupLayout) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cascade matrix"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow depth"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/shadow_depth.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sun depth"),
            bind_group_layouts: &[Some(&layout), Some(texture)],
            immediate_size: 0,
        });
        let make = |entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(wgpu::VertexBufferLayout {
                            array_stride: size_of::<Vertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &VERTEX_ATTRIBUTES,
                        }),
                        Some(INSTANCE_LAYOUT),
                    ],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState { constant: 1, slope_scale: 1.0, clamp: 0.0 },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let buffers: Vec<_> = (0..4)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("cascade matrix"),
                    size: 64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let groups = buffers
            .iter()
            .map(|buffer| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("cascade matrix"),
                    layout: &layout,
                    entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
                })
            })
            .collect();
        Self {
            opaque: make("fs_opaque"),
            cutout: make("fs_cutout"),
            buffers,
            groups,
            instances: (0..4).map(|_| crate::gpu::instances::InstanceBuffer::new(device)).collect(),
            bounds: Vec::new(),
            selected: Vec::new(),
            matrices: Vec::new(),
        }
    }
}
