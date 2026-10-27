use super::atlas::Atlas;
use wgpu::util::DeviceExt;
pub(super) const MAX_INSTANCES: usize = 32768;
// Fourteen flag bits encode the shading family/material slot; values 0..2 are non-glossy.
pub(super) const MAX_MATERIALS: usize = (1 << 14) - 3;
pub(in crate::gpu) const MATERIAL_STRIDE: usize = 112;
pub(super) fn tlas(device: &wgpu::Device, capacity: usize) -> wgpu::Tlas {
    device.create_tlas(&wgpu::CreateTlasDescriptor {
        label: Some("RT reflection TLAS"),
        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        max_instances: capacity as u32,
    })
}
#[derive(Clone)]
pub(in crate::gpu) struct Bindings {
    pub tlas: wgpu::Tlas,
    pub vertices: wgpu::Buffer,
    pub triangles: wgpu::Buffer,
    pub atlas: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub control: wgpu::Buffer,
    pub winding: wgpu::Buffer,
    pub materials: wgpu::Buffer,
    pub detail: wgpu::TextureView,
}
pub(super) fn storage(device: &wgpu::Device, label: &str, bytes: &[u8]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: if bytes.is_empty() { &[0; 64] } else { bytes },
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    })
}
impl Bindings {
    pub(super) fn new(device: &wgpu::Device, atlas: &Atlas, detail: &Atlas) -> Self {
        let tlas = tlas(device, 1);
        Self {
            tlas,
            vertices: storage(device, "RT empty vertices", &[]),
            triangles: storage(device, "RT empty triangles", &[]),
            atlas: atlas.view.clone(),
            detail: detail.view.clone(),
            sampler: atlas.sampler.clone(),
            materials: storage(device, "RT hit materials", &vec![0; MAX_MATERIALS * MATERIAL_STRIDE]),
            winding: storage(device, "RT instance winding", &vec![0; MAX_INSTANCES * 4]),
            control: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RT scene control"),
                contents: &[0; 32],
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
        }
    }
    pub fn entries<'a>(&'a self, globals: &'a wgpu::Buffer) -> [wgpu::BindGroupEntry<'a>; 10] {
        [
            wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: self.tlas.as_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: self.vertices.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: self.triangles.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&self.atlas) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            wgpu::BindGroupEntry { binding: 6, resource: self.control.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 7, resource: self.winding.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 8, resource: self.materials.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&self.detail) },
        ]
    }
}
pub(in crate::gpu) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 10] {
    let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    };
    [
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::AccelerationStructure { vertex_return: false },
            count: None,
        },
        buffer(2, wgpu::BufferBindingType::Storage { read_only: true }),
        buffer(3, wgpu::BufferBindingType::Storage { read_only: true }),
        wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
        buffer(6, wgpu::BufferBindingType::Uniform),
        buffer(7, wgpu::BufferBindingType::Storage { read_only: true }),
        buffer(8, wgpu::BufferBindingType::Storage { read_only: true }),
        wgpu::BindGroupLayoutEntry {
            binding: 9,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
    ]
}
