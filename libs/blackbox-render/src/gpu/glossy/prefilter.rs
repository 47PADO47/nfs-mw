//! Bounded GGX environment filtering; raw reflections remain available for legacy draws.
use wgpu::util::DeviceExt;

#[path = "prefilter_source.rs"]
mod source;
#[cfg(test)]
#[path = "prefilter_tests.rs"]
mod tests;
const SIZE: u32 = 128;
const LEVELS: u32 = 8;
pub(super) const SHADER: &str = include_str!("../../shaders/environment_prefilter.wgsl");

pub(super) struct Prefilter {
    pub view: wgpu::TextureView,
    pub linear_view: wgpu::TextureView,
    texture: wgpu::Texture,
    pipeline: Option<wgpu::ComputePipeline>,
    cache: Option<wgpu::PipelineCache>,
    /// One bind group per level, with its output view and parameters. Reused while the source and sampler stay.
    groups: Vec<wgpu::BindGroup>,
    built_for: Option<(wgpu::TextureView, wgpu::Sampler)>,
    pub dirty: bool,
    source: Option<source::Source>,
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
        Some(Self {
            view,
            linear_view,
            texture,
            pipeline: None,
            cache: cache.cloned(),
            groups: Vec::new(),
            built_for: None,
            dirty: true,
            source: None,
        })
    }

    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
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
        let input = self.source.get_or_insert_with(|| source::Source::new(device, self.cache.as_ref()));
        let current = self.built_for.as_ref().is_some_and(|(view, made)| view == source && made == sampler);
        if !current {
            input.bind(device, source, sampler);
            let texture = &self.texture;
            self.groups =
                (0..LEVELS).map(|level| level_group(device, texture, pipeline, input, sampler, level)).collect();
            self.built_for = Some((source.clone(), sampler.clone()));
        }
        input.encode(encoder);
        for (level, group) in self.groups.iter().enumerate() {
            let size = SIZE >> level;
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("filter environment level"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(size.div_ceil(8), size.div_ceil(8), 12);
        }
        self.dirty = false;
    }
}

/// The inputs of one level: the source cube, the level's own output view and its constant parameters.
fn level_group(
    device: &wgpu::Device,
    texture: &wgpu::Texture,
    pipeline: &wgpu::ComputePipeline,
    source: &source::Source,
    sampler: &wgpu::Sampler,
    level: u32,
) -> wgpu::BindGroup {
    let size = SIZE >> level;
    let output = texture.create_view(&wgpu::TextureViewDescriptor {
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
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("environment filter inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&source.encoded) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&output) },
            wgpu::BindGroupEntry { binding: 3, resource: params.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&source.linear) },
        ],
    })
}
