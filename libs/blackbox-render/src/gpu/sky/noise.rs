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
                mip_level_count: size.ilog2() + 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: 0,
                mip_level_count: Some(1),
                ..Default::default()
            });
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
            mips(device, &mut encoder, &texture);
            texture.create_view(&Default::default())
        });
    queue.submit([encoder.finish()]);
    views
}

fn mips(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, texture: &wgpu::Texture) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("cloud density mip filter"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/sky_filter.wgsl").into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("cloud density mip filter"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    for level in 1..texture.mip_level_count() {
        let view = |mip| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let source = view(level - 1);
        let target = view(level);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud density mip filter"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&source) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&target) },
            ],
        });
        let size = texture.size();
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(
            (size.width >> level).max(1).div_ceil(4),
            (size.height >> level).max(1).div_ceil(4),
            (size.depth_or_array_layers >> level).max(1),
        );
    }
}

#[cfg(test)]
#[path = "noise_tests.rs"]
mod tests;
