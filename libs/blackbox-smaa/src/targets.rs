//! The sized render targets of an SMAA frame: the scene colour target, the edges and the blend weights, and the
//! metrics uniform the shaders read the texel size from. The pure helper [`rt_metrics`] is the uniform's contents.

use wgpu::util::DeviceExt;

/// The sized targets. Rebuilt on resize.
pub(crate) struct Targets {
    /// `(1/width, 1/height, width, height)`, read by every pass.
    pub(crate) rt_uniforms: wgpu::Buffer,
    /// The scene is rendered here, in the output format.
    pub(crate) color: wgpu::TextureView,
    /// Pass 1 output: the edges (RG8).
    pub(crate) edges: wgpu::TextureView,
    /// Pass 2 output: the blend weights (RGBA8).
    pub(crate) blend: wgpu::TextureView,
    /// Stencil mask: 1 where the edge pass found an edge, so the blend-weight pass can skip the other pixels.
    pub(crate) stencil: wgpu::TextureView,
}

impl Targets {
    pub(crate) fn new(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        // A zero-sized surface (a minimised window) still needs targets, and the metrics divide by the size.
        let (width, height) = (width.max(1), height.max(1));
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let contents: Vec<u8> = rt_metrics(width, height).iter().flat_map(|value| value.to_ne_bytes()).collect();
        let rt_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("smaa.uniforms"),
            usage: wgpu::BufferUsages::UNIFORM,
            contents: &contents,
        });
        Self {
            rt_uniforms,
            color: target_view(device, "smaa.color_target", size, format),
            edges: target_view(device, "smaa.texture.edge_target", size, wgpu::TextureFormat::Rg8Unorm),
            blend: target_view(device, "smaa.texture.blend_target", size, wgpu::TextureFormat::Rgba8Unorm),
            stencil: target_view(device, "smaa.texture.stencil_mask", size, wgpu::TextureFormat::Stencil8),
        }
    }
}

/// The metrics uniform of the reference (`SMAA_RT_METRICS`): `(1/width, 1/height, width, height)`.
pub(crate) fn rt_metrics(width: u32, height: u32) -> [f32; 4] {
    let (w, h) = (width as f32, height as f32);
    [1.0 / w, 1.0 / h, w, h]
}

fn target_view(
    device: &wgpu::Device,
    label: &str,
    size: wgpu::Extent3d,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor { label: Some(label), ..Default::default() })
}
