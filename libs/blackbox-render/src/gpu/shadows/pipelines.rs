use super::Shadows;
use crate::Vertex;
use crate::gpu::draw_list::{instance_store, instance_view};
use crate::gpu::pipelines::VERTEX_ATTRIBUTES;

impl Shadows {
    pub(super) fn new(
        device: &wgpu::Device,
        texture: &wgpu::BindGroupLayout,
        bias: [f32; 2],
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
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
        let store = instance_store::supported(device);
        let instances = store.then(|| instance_store::layout(device));
        let source =
            instance_view::instance_source(include_str!("../../shaders/shadow_depth.wgsl"), store.then_some(2));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow depth"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sun depth"),
            bind_group_layouts: &[Some(&layout), Some(texture), instances.as_ref()][..2 + usize::from(store)],
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
                        Some(instance_view::instance_layout(store)),
                    ],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState { constant: bias[0] as i32, slope_scale: bias[1], clamp: 0.0 },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                multiview_mask: None,
                cache,
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
            grid: super::grid::Grid::default(),
            cache: super::cache::Cache::default(),
            static_instances: (0..4).map(|_| crate::gpu::instances::InstanceBuffer::new(device)).collect(),
            bias,
            opaque: make("fs_opaque"),
            cutout: make("fs_cutout"),
            tree_cutout: make("fs_tree_cutout"),
            prelit_cutout: make("fs_prelit_cutout"),
            buffers,
            groups,
            instances: (0..4).map(|_| crate::gpu::instances::InstanceBuffer::new(device)).collect(),
            bounds: Vec::new(),
            selected: Vec::new(),
            scratch: Default::default(),
            caster_slots: Vec::new(),
            store,
            fit_points: Vec::new(),
        }
    }
}
