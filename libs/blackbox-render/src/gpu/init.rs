//! Instance, surface, adapter and device setup.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::{Renderer, instances, pipelines, post, resources, slots::Slots, targets};
use crate::{Backend, DEFAULT_RENDER_SCALE, PixelFormat, RenderError, RendererOptions, TextureDesc};

fn wgpu_backends(backend: Backend) -> wgpu::Backends {
    match backend {
        Backend::Auto => wgpu::Backends::PRIMARY | wgpu::Backends::GL,
        Backend::Vulkan => wgpu::Backends::VULKAN,
        Backend::Dx12 => wgpu::Backends::DX12,
        Backend::Gl => wgpu::Backends::GL,
    }
}

pub(super) fn create<W>(
    window: W,
    size: (u32, u32),
    display: impl HasDisplayHandle + std::fmt::Debug + Send + Sync + 'static,
    options: RendererOptions,
) -> Result<Renderer, RenderError>
where
    W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
{
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle().with_display_handle(Box::new(display));
    desc.backends = wgpu_backends(options.backend);
    let instance = wgpu::Instance::new(desc);
    let surface = instance.create_surface(window).map_err(|e| RenderError::Surface(e.to_string()))?;

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
        apply_limit_buckets: false,
    }))
    .map_err(|e| RenderError::NoAdapter { backend: options.backend, detail: e.to_string() })?;
    let adapter_info = adapter.get_info();
    log::info!("GPU: {} ({:?}, driver {})", adapter_info.name, adapter_info.backend, adapter_info.driver);

    let supports_bc = adapter.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
    let required_features = if supports_bc { wgpu::Features::TEXTURE_COMPRESSION_BC } else { wgpu::Features::empty() };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("blackbox-render device"),
        required_features,
        required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    }))
    .map_err(|e| RenderError::Device(e.to_string()))?;

    let mut config = surface
        .get_default_config(&adapter, size.0.max(1), size.1.max(1))
        .ok_or_else(|| RenderError::Surface("the surface is not supported by this adapter".into()))?;
    // The games' art is authored for a non-sRGB D3D9 pipeline: prefer a plain UNORM target.
    let caps = surface.get_capabilities(&adapter);
    if let Some(f) = caps.formats.iter().copied().find(|f| !f.is_srgb()) {
        config.format = f;
    }
    config.present_mode = if options.vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
    surface.configure(&device, &config);

    let shared = resources::Shared::new(&device);
    // With nothing but the plain resolve in the chain the scene draws straight into the surface, like
    // the UI; passes that need the offscreen image (see `refresh_targets`) switch it on later.
    let hdr_format = targets::pick_color_format(&adapter, config.format);
    let output = (config.width, config.height);
    let plan = targets::plan_scene(&targets::SceneInputs {
        output,
        render: output,
        passes: false,
        hdr: false,
        surface_format: config.format,
        hdr_format,
    });
    let pipelines = pipelines::Pipelines::new(&device, plan.format, &shared);
    let targets = targets::SceneTargets::new(&device, &plan);
    let post = post::PostChain::new(&device);
    let instances = instances::InstanceBuffer::new(&device);
    let ui = super::ui::Ui::new(&device, config.format, &shared);
    let effects = super::effects::Effects::new(&device, plan.format, &shared);

    let mut renderer = Renderer {
        surface,
        device,
        queue,
        config,
        adapter_info,
        supports_bc,
        targets,
        hdr_format,
        post,
        render_scale: DEFAULT_RENDER_SCALE,
        upscale: Default::default(),
        shared,
        pipelines,
        textures: Slots::new(),
        meshes: Slots::new(),
        instances,
        effects,
        glossy: None,
        ui,
        redirects: Default::default(),
    };
    let white = [255u8; 4];
    renderer.create_texture(&TextureDesc {
        label: "white",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&white],
    });
    Ok(renderer)
}
