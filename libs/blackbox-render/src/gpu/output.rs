//! Where finished frames go: a window's surface, or a plain texture for headless use and tests.

/// The format of a headless output texture: plain UNORM, like the surface the games' art is authored for.
pub(super) const HEADLESS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub(super) enum Output {
    /// A window's swapchain.
    Window { surface: wgpu::Surface<'static>, config: wgpu::SurfaceConfiguration },
    /// A texture nothing presents; the frame ends up in it. No window system is involved.
    Texture { texture: wgpu::Texture, size: (u32, u32) },
}

impl Output {
    /// A texture of `size` (at least 1x1) in [`HEADLESS_FORMAT`] that frames can be drawn to and copied from.
    pub(super) fn texture(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let size = (size.0.max(1), size.1.max(1));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless output"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HEADLESS_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self::Texture { texture, size }
    }

    pub(super) fn size(&self) -> (u32, u32) {
        match self {
            Self::Window { config, .. } => (config.width, config.height),
            Self::Texture { size, .. } => *size,
        }
    }

    pub(super) fn format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Window { config, .. } => config.format,
            Self::Texture { .. } => HEADLESS_FORMAT,
        }
    }

    /// Follow a new size of at least 1x1.
    pub(super) fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        match self {
            Self::Window { surface, config } => {
                config.width = size.0;
                config.height = size.1;
                surface.configure(device, config);
            }
            Self::Texture { .. } => *self = Self::texture(device, size),
        }
    }

    /// Switch vertical sync; a texture has none.
    pub(super) fn set_vsync(&mut self, device: &wgpu::Device, vsync: bool) {
        let Self::Window { surface, config } = self else { return };
        config.present_mode = if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        surface.configure(device, config);
    }
}
