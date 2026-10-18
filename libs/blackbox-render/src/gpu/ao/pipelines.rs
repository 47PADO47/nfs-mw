//! Format-independent GTAO generation, spatial filtering and R8 visibility resolve.
pub(super) struct Pipelines {
    pub generate: wgpu::ComputePipeline,
    pub ssao: wgpu::ComputePipeline,
    pub xegtao: wgpu::ComputePipeline,
    pub xe_filter: wgpu::ComputePipeline,
    pub xe_intermediate: wgpu::ComputePipeline,
    pub filter: wgpu::ComputePipeline,
    pub resolve: wgpu::RenderPipeline,
    pub depth_first: wgpu::ComputePipeline,
    pub depth_reduce: wgpu::ComputePipeline,
    pub depth_layout: wgpu::BindGroupLayout,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        let sources = super::sources();
        let shaders = sources.map(|(label, source)| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        });
        let uniform = entry(
            0,
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        );
        let depth_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("AO depth prefilter layout"),
            entries: &[
                uniform,
                sampled(1, wgpu::TextureSampleType::Depth),
                sampled(2, wgpu::TextureSampleType::Float { filterable: false }),
                storage(3, wgpu::TextureFormat::R32Float),
            ],
        });
        let generate_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("AO horizon layout"),
            entries: &[
                uniform,
                sampled(1, wgpu::TextureSampleType::Depth),
                storage(2, wgpu::TextureFormat::Rgba16Float),
                sampled(4, wgpu::TextureSampleType::Float { filterable: true }),
                sampled(5, wgpu::TextureSampleType::Float { filterable: false }),
            ],
        });
        let layout = |label, group: &wgpu::BindGroupLayout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(group)],
                immediate_size: 0,
            })
        };
        let filter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("AO spatial filter layout"),
            entries: &[
                uniform,
                sampled(1, wgpu::TextureSampleType::Depth),
                sampled(2, wgpu::TextureSampleType::Float { filterable: true }),
                storage(3, wgpu::TextureFormat::Rgba16Float),
                sampled(4, wgpu::TextureSampleType::Float { filterable: true }),
            ],
        });
        let filter_pipeline_layout = layout("AO spatial filter", &filter_layout);
        let depth_pipeline_layout = layout("AO depth prefilter", &depth_layout);
        let generate_pipeline_layout = layout("AO horizons", &generate_layout);
        let depth_pipeline = |first: bool| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("AO depth prefilter"),
                layout: Some(&depth_pipeline_layout),
                module: &shaders[3],
                entry_point: Some("reduce_depth"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[("FROM_SCENE", f64::from(u8::from(first)))],
                    ..Default::default()
                },
                cache,
            })
        };
        let compute = |index: usize, entry, layout: &wgpu::PipelineLayout| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(layout),
                module: &shaders[index],
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache,
            })
        };
        Self {
            xe_intermediate: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("XeGTAO intermediate spatial denoise"),
                layout: Some(&filter_pipeline_layout),
                module: &shaders[4],
                entry_point: Some("denoise_xe"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[("FINAL_PASS", 0.0)],
                    ..Default::default()
                },
                cache,
            }),
            xegtao: compute(0, "generate_xegtao", &generate_pipeline_layout),
            xe_filter: compute(4, "denoise_xe", &filter_pipeline_layout),
            ssao: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("AO hemisphere kernel"),
                layout: Some(&generate_pipeline_layout),
                module: &shaders[0],
                entry_point: Some("generate_ssao"),
                compilation_options: Default::default(),
                cache,
            }),
            generate: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("AO horizons"),
                layout: Some(&generate_pipeline_layout),
                module: &shaders[0],
                entry_point: Some("generate"),
                compilation_options: Default::default(),
                cache,
            }),
            depth_first: depth_pipeline(true),
            depth_reduce: depth_pipeline(false),
            depth_layout,
            filter: compute(1, "denoise", &filter_pipeline_layout),
            resolve: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ambient visibility resolve"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shaders[2],
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shaders[2],
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::RED,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache,
            }),
        }
    }
}

fn entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE, ty, count: None }
}
fn sampled(binding: u32, sample_type: wgpu::TextureSampleType) -> wgpu::BindGroupLayoutEntry {
    entry(
        binding,
        wgpu::BindingType::Texture { sample_type, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
    )
}
fn storage(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    entry(
        binding,
        wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
    )
}
