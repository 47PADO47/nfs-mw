//! Low-frequency diffuse illumination from the current cube; no pixel history or CPU readback.
pub(super) struct Irradiance {
    pub buffer: wgpu::Buffer,
    pipeline: Option<wgpu::ComputePipeline>,
    pub dirty: bool,
    /// Used by the pipeline, which is built the first time the cube is projected.
    cache: Option<wgpu::PipelineCache>,
    /// The inputs of the last update with the cube and sampler they were made for; reused while those stay.
    inputs: Option<(wgpu::TextureView, wgpu::Sampler, wgpu::BindGroup)>,
}

impl Irradiance {
    pub fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sky and ground irradiance SH"),
                size: 18 * 16,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            pipeline: None,
            dirty: true,
            cache: cache.cloned(),
            inputs: None,
        }
    }

    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        cube: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) {
        if !self.dirty {
            return;
        }
        let pipeline = self.pipeline.get_or_insert_with(|| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("diffuse environment projection"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/irradiance.wgsl").into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("diffuse environment projection"),
                layout: None,
                module: &shader,
                entry_point: Some("project"),
                compilation_options: Default::default(),
                cache: self.cache.as_ref(),
            })
        });
        let current = self.inputs.as_ref().is_some_and(|(view, made, _)| view == cube && made == sampler);
        if !current {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("diffuse environment projection"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(cube) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: self.buffer.as_entire_binding() },
                ],
            });
            self.inputs = Some((cube.clone(), sampler.clone(), group));
        }
        let Some((_, _, inputs)) = &self.inputs else { return };
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("diffuse probe update"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, inputs, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        self.dirty = false;
    }
}
