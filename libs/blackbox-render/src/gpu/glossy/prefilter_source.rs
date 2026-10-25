//! Spatially antialiased input for GGX integration. Original capture texels stay untouched.
use wgpu::util::DeviceExt;

pub(super) const SHADER: &str = include_str!("../../shaders/environment_source.wgsl");
const SIZE: u32 = 256;
const LEVELS: u32 = 9;

pub(super) struct Source {
    pub encoded: wgpu::TextureView,
    pub linear: wgpu::TextureView,
    seed: wgpu::ComputePipeline,
    reduce: wgpu::ComputePipeline,
    groups: Vec<wgpu::BindGroup>,
    texture: wgpu::Texture,
}

impl Source {
    pub fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("reflection integration source"),
            size: wgpu::Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 12 },
            mip_level_count: LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let encoded = cube_view(&texture, 0);
        let linear = cube_view(&texture, 6);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("reflection source pyramid"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache,
            })
        };
        Self { encoded, linear, seed: pipeline("seed"), reduce: pipeline("reduce"), groups: Vec::new(), texture }
    }

    pub fn bind(&mut self, device: &wgpu::Device, source: &wgpu::TextureView, sampler: &wgpu::Sampler) {
        self.groups = (0..LEVELS)
            .map(|level| {
                let output = self.texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                });
                let previous = self.texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    base_mip_level: level.saturating_sub(1),
                    mip_level_count: Some(1),
                    ..Default::default()
                });
                let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("source footprint"),
                    contents: bytemuck::cast_slice(&[SIZE >> level, 0, 0, 0]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let (pipeline, binding, input) = match level {
                    0 => (&self.seed, 0, source),
                    _ => (&self.reduce, 4, &previous),
                };
                let mut entries = vec![
                    wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(input) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&output) },
                    wgpu::BindGroupEntry { binding: 3, resource: params.as_entire_binding() },
                ];
                if level == 0 {
                    entries
                        .push(wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) });
                }
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("reflection source level"),
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &entries,
                })
            })
            .collect();
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        for (level, group) in self.groups.iter().enumerate() {
            let size = SIZE >> level;
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("reflection source footprint"),
                timestamp_writes: None,
            });
            pass.set_pipeline(match level {
                0 => &self.seed,
                _ => &self.reduce,
            });
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(size.div_ceil(8), size.div_ceil(8), 12);
        }
    }
}

fn cube_view(texture: &wgpu::Texture, base_array_layer: u32) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        base_array_layer,
        array_layer_count: Some(6),
        ..Default::default()
    })
}
