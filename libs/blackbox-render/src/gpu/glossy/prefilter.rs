//! Bounded GGX environment filtering; raw reflections remain available for legacy draws.
use wgpu::util::DeviceExt;

const SIZE: u32 = 64;
const LEVELS: u32 = 7;
pub(super) const SHADER: &str = include_str!("../../shaders/environment_prefilter.wgsl");

pub(super) struct Prefilter {
    pub view: wgpu::TextureView,
    pub linear_view: wgpu::TextureView,
    texture: wgpu::Texture,
    pipeline: Option<wgpu::ComputePipeline>,
    cache: Option<wgpu::PipelineCache>,
    pub dirty: bool,
}

impl Prefilter {
    pub fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Option<Self> {
        let limits = device.limits();
        if limits.max_compute_invocations_per_workgroup < 64 || limits.max_storage_textures_per_shader_stage == 0 {
            return None;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("roughness filtered environment"),
            size: wgpu::Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 12 },
            mip_level_count: LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            array_layer_count: Some(6),
            ..Default::default()
        });
        let linear_view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            base_array_layer: 6,
            array_layer_count: Some(6),
            ..Default::default()
        });
        Some(Self { view, linear_view, texture, pipeline: None, cache: cache.cloned(), dirty: true })
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) {
        if !self.dirty {
            return;
        }
        let pipeline = self.pipeline.get_or_insert_with(|| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("GGX environment filter"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("GGX environment filter"),
                layout: None,
                module: &module,
                entry_point: Some("prefilter_main"),
                compilation_options: Default::default(),
                cache: self.cache.as_ref(),
            })
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("specular probe update") });
        for level in 0..LEVELS {
            let size = SIZE >> level;
            let output = self.texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("environment filter level"),
                contents: bytemuck::cast_slice(&[level as f32 / (LEVELS - 1) as f32, size as f32, 0.0, 0.0]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("environment filter inputs"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(source) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&output) },
                    wgpu::BindGroupEntry { binding: 3, resource: params.as_entire_binding() },
                ],
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("filter environment level"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(size.div_ceil(8), size.div_ceil(8), 12);
        }
        queue.submit([encoder.finish()]);
        self.dirty = false;
    }
}
