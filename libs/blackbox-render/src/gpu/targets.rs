//! The offscreen frame targets the scene is drawn into: an HDR colour image and a depth buffer,
//! both at the internal render size.

use super::resources::create_depth;

/// The preferred HDR colour format of the offscreen scene image.
pub(super) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The format the scene is drawn in: [`HDR_FORMAT`] when the adapter can render to it, blend into
/// it and sample it with a filtering sampler, otherwise the surface's own `fallback` format (the
/// frame then looks exactly as it did before the HDR target existed).
pub(super) fn pick_color_format(adapter: &wgpu::Adapter, fallback: wgpu::TextureFormat) -> wgpu::TextureFormat {
    let features = adapter.get_texture_format_features(HDR_FORMAT);
    let usages = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    let flags = wgpu::TextureFormatFeatureFlags::FILTERABLE | wgpu::TextureFormatFeatureFlags::BLENDABLE;
    if features.allowed_usages.contains(usages) && features.flags.contains(flags) {
        return HDR_FORMAT;
    }
    log::warn!("{HDR_FORMAT:?} is not renderable here; the scene is drawn in {fallback:?} instead");
    fallback
}

/// The scene's colour image and depth buffer. Post-process passes read both.
pub(super) struct FrameTargets {
    _color: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub color_format: wgpu::TextureFormat,
    /// Reverse-Z depth (1 near, 0 far). Also bindable as a sampled depth texture.
    pub depth_view: wgpu::TextureView,
    pub size: (u32, u32),
}

impl FrameTargets {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, size: (u32, u32)) -> Self {
        let size = (size.0.max(1), size.1.max(1));
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene colour"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = create_depth(device, size.0, size.1);
        Self { _color: color, color_view, color_format: format, depth_view, size }
    }
}
