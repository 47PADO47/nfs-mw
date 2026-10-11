//! The three AO pipelines. Only the composite depends on the scene's colour format.

pub(super) struct Pipelines {
    pub generate: wgpu::ComputePipeline,
    pub filter: wgpu::ComputePipeline,
    pub composite: Option<wgpu::RenderPipeline>,
    format: Option<wgpu::TextureFormat>,
    composite_shader: wgpu::ShaderModule,
}

impl Pipelines {
    pub fn new(device: &wgpu::Device) -> Self {
        let sources = super::sources();
        let compute = |index: usize, entry| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(sources[index].0),
                source: wgpu::ShaderSource::Wgsl(sources[index].1.as_str().into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(sources[index].0),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Self {
            generate: compute(0, "generate"),
            filter: compute(1, "denoise"),
            composite: None,
            format: None,
            composite_shader: device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(sources[2].0),
                source: wgpu::ShaderSource::Wgsl(sources[2].1.as_str().into()),
            }),
        }
    }

    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        if self.format == Some(format) {
            return;
        }
        self.composite = Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("AO multiply"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &self.composite_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.composite_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Dst,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::RED | wgpu::ColorWrites::GREEN | wgpu::ColorWrites::BLUE,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        }));
        self.format = Some(format);
    }
}
