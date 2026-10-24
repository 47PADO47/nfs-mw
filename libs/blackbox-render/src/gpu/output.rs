//! Where finished frames go: a window's surface, or a plain texture for headless use and tests.

/// The format of a headless output texture: plain UNORM, like the surface the games' art is authored for.
pub(super) const HEADLESS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub(super) enum Output {
    /// A window's swapchain.
    Window {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        /// The surface offers `Mailbox`, the present mode without vsync that never tears and never waits.
        mailbox: bool,
        #[cfg(all(windows, feature = "streamline"))]
        dx12: Option<super::dlss::dx12::Surface>,
    },
    /// A texture nothing presents; the frame ends up in it. No window system is involved.
    Texture { texture: wgpu::Texture, size: (u32, u32) },
}

impl Output {
    /// A texture of `size` (at least 1x1) in [`HEADLESS_FORMAT`] that frames can be drawn to and copied from.
    pub(super) fn texture(device: &wgpu::Device, size: (u32, u32)) -> Self {
        Self::texture_format(device, size, HEADLESS_FORMAT)
    }
    pub(super) fn texture_format(device: &wgpu::Device, size: (u32, u32), format: wgpu::TextureFormat) -> Self {
        let size = (size.0.max(1), size.1.max(1));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless output"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
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
            Self::Texture { texture, .. } => texture.format(),
        }
    }

    pub(super) fn configure(&mut self, device: &wgpu::Device) {
        #[cfg(all(windows, feature = "streamline"))]
        if let Self::Window { dx12: Some(native), config, .. } = self {
            native.configure(device, config);
            return;
        }
        let Self::Window { surface, config, .. } = self else { return };
        surface.configure(device, config);
    }
    /// Follow a new size of at least 1x1.
    pub(super) fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        match self {
            Self::Window { config, .. } => {
                config.width = size.0;
                config.height = size.1;
                self.configure(device);
            }
            Self::Texture { texture, .. } => *self = Self::texture_format(device, size, texture.format()),
        }
    }

    /// Switch vertical sync; a texture has none.
    pub(super) fn set_vsync(&mut self, device: &wgpu::Device, vsync: bool) {
        let Self::Window { config, mailbox, .. } = self else { return };
        config.present_mode = present_mode(vsync, *mailbox);
        self.configure(device);
    }

    /// Frames the presentation engine may queue ahead (1 or 2); a texture has no queue.
    pub(super) fn set_frame_latency(&mut self, device: &wgpu::Device, frames: u32) {
        let Self::Window { config, .. } = self else { return };
        let frames = frames.clamp(1, MAX_FRAME_LATENCY);
        if config.desired_maximum_frame_latency == frames {
            return;
        }
        config.desired_maximum_frame_latency = frames;
        self.configure(device);
    }

    /// Whether the output can be copied from and to, which showing a kept image needs (`reuse.rs`). A window's
    /// swapchain only takes part when the surface allowed both usages; the native Direct3D 12 swapchain never does.
    pub(super) fn copyable(&self) -> bool {
        let Self::Window { config, .. } = self else { return true };
        #[cfg(all(windows, feature = "streamline"))]
        if matches!(self, Self::Window { dx12: Some(_), .. }) {
            return false;
        }
        config.usage.contains(COPY_USAGES)
    }
}

/// The most frames the presentation engine queues ahead that the renderer asks for; wgpu's own default.
pub(super) const MAX_FRAME_LATENCY: u32 = 2;

/// What a swapchain also needs to be copied from and to.
pub(super) const COPY_USAGES: wgpu::TextureUsages = wgpu::TextureUsages::COPY_SRC.union(wgpu::TextureUsages::COPY_DST);

/// The present mode for the vsync setting. Without vsync `Mailbox` shows the newest finished frame at each refresh
/// without tearing, when the surface has it; otherwise the platform's own no-vsync mode.
pub(super) fn present_mode(vsync: bool, mailbox: bool) -> wgpu::PresentMode {
    if vsync {
        return wgpu::PresentMode::AutoVsync;
    }
    if mailbox {
        return wgpu::PresentMode::Mailbox;
    }
    wgpu::PresentMode::AutoNoVsync
}

impl super::Renderer {
    /// How many frames the presentation engine may queue ahead of the display: 2 (the default) keeps the GPU busy,
    /// 1 shows each frame sooner at the cost of stalls when the CPU is late. Clamped to 1..=2.
    pub fn set_max_frame_latency(&mut self, frames: u32) {
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            self.presentation.suspend(&self.device);
        }
        self.output.set_frame_latency(&self.device, frames);
    }
}
