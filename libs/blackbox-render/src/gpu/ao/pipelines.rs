//! Format-independent GTAO generation, spatial filtering and R8 visibility resolve.
pub(super) struct Pipelines {
    pub generate: wgpu::ComputePipeline,
    pub filter: wgpu::ComputePipeline,
    pub resolve: wgpu::RenderPipeline,
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
        let compute = |index: usize, entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shaders[index],
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache,
            })
        };
        Self {
            generate: compute(0, "generate"),
            filter: compute(1, "denoise"),
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
