//! Bind group layouts of the glossy pipelines: group 2 holds the scene's rig and environment,
//! group 3 one material.

#[derive(Clone)]
pub(in crate::gpu) struct Layouts {
    /// Group 2: the lighting rig, the environment cube map and its sampler.
    pub scene: wgpu::BindGroupLayout,
    /// Group 3: one material's constants.
    pub material: wgpu::BindGroupLayout,
}

fn uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl Layouts {
    pub(in crate::gpu) fn new(device: &wgpu::Device) -> Self {
        let scene = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glossy scene"),
            entries: &[
                uniform(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let material = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glossy material"),
            entries: &[uniform(0)],
        });
        Self { scene, material }
    }
}
