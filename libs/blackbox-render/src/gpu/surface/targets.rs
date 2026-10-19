//! Scene-resolution depth and world normals. No previous-frame data.
// UNORM8 cannot represent an encoded zero component (0.5) exactly; its decoded
// tilt makes long AO samples falsely self-occlude flat surfaces.
pub(super) const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub(in crate::gpu) struct Targets {
    pub size: (u32, u32),
    pub depth: wgpu::TextureView,
    pub normals: wgpu::TextureView,
}

impl Targets {
    pub(in crate::gpu) fn with_depth(device: &wgpu::Device, size: (u32, u32), depth: wgpu::TextureView) -> Self {
        let make = |format, label| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        Self { size, depth, normals: make(NORMAL_FORMAT, "surface normals") }
    }
    pub(in crate::gpu) fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        Self::with_depth(device, size, crate::gpu::resources::create_depth(device, size.0, size.1))
    }
}
