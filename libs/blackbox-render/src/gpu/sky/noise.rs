//! One-time GPU generation of periodic base and erosion fields.

pub(super) fn create(device: &wgpu::Device, queue: &wgpu::Queue) -> [wgpu::TextureView; 3] {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("cloud noise generation"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/sky_noise.wgsl").into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("cloud noise output"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: wgpu::TextureFormat::Rgba8Unorm,
                view_dimension: wgpu::TextureViewDimension::D3,
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("cloud noise generation"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    let views =
        [(128, 128, "base_main"), (32, 32, "detail_main"), (128, 1, "weather_main")].map(|(size, depth, entry)| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(entry),
                size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: depth },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(entry),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }],
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            });
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups(size / 8, size / 8, depth);
            }
            view
        });
    queue.submit([encoder.finish()]);
    views
}
