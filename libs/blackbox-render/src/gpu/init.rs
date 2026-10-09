//! Instance, surface, adapter and device setup.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::output::Output;
use super::{Renderer, capture, instances, pipelines, post, resources, slots::Slots, targets};
use crate::caps::native_capabilities;
use crate::{Backend, DEFAULT_RENDER_SCALE, PixelFormat, RenderError, RendererOptions, TextureDesc};
use blackbox_gfx::{BackendInfo, GraphicsApi, GraphicsSettings};

fn wgpu_backends(backend: Backend) -> wgpu::Backends {
    match backend {
        Backend::Auto => wgpu::Backends::PRIMARY | wgpu::Backends::GL,
        Backend::Vulkan => wgpu::Backends::VULKAN,
        Backend::Dx12 => wgpu::Backends::DX12,
        Backend::Gl => wgpu::Backends::GL,
    }
}

fn api_of(backend: wgpu::Backend) -> GraphicsApi {
    match backend {
        wgpu::Backend::Vulkan => GraphicsApi::Vulkan,
        wgpu::Backend::Dx12 => GraphicsApi::Dx12,
        wgpu::Backend::Gl => GraphicsApi::Gl,
        _ => GraphicsApi::Auto,
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

    let adapter = request_adapter(&instance, Some(&surface), options)?;
    let (device, queue) = request_device(&adapter)?;

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

    Ok(assemble(Output::Window { surface, config }, &adapter, device, queue))
}

pub(super) fn create_headless(size: (u32, u32), options: RendererOptions) -> Result<Renderer, RenderError> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu_backends(options.backend);
    let instance = wgpu::Instance::new(desc);
    let adapter = request_adapter(&instance, None, options)?;
    let (device, queue) = request_device(&adapter)?;
    let output = Output::texture(&device, size);
    Ok(assemble(output, &adapter, device, queue))
}

fn request_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    options: RendererOptions,
) -> Result<wgpu::Adapter, RenderError> {
    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: options.force_fallback_adapter,
        compatible_surface: surface,
        apply_limit_buckets: false,
    }))
    .map_err(|e| RenderError::NoAdapter { backend: options.backend, detail: e.to_string() })
}

fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), RenderError> {
    let info = adapter.get_info();
    log::info!("GPU: {} ({:?}, driver {})", info.name, info.backend, info.driver);
    let supports_bc = adapter.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
    let required_features = if supports_bc { wgpu::Features::TEXTURE_COMPRESSION_BC } else { wgpu::Features::empty() };
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("blackbox-render device"),
        required_features,
        required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
        ..Default::default()
    }))
    .map_err(|e| RenderError::Device(e.to_string()))
}

/// Build the renderer around a ready device and output.
fn assemble(output: Output, adapter: &wgpu::Adapter, device: wgpu::Device, queue: wgpu::Queue) -> Renderer {
    let adapter_info = adapter.get_info();
    let supports_bc = device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
    let shared = resources::Shared::new(&device);
    // With nothing but the plain resolve in the chain the scene draws straight into the output, like
    // the UI; passes that need the offscreen image (see `refresh_targets`) switch it on later.
    let format = output.format();
    let hdr_format = targets::pick_color_format(adapter, format);
    let size = output.size();
    let plan = targets::plan_scene(&targets::SceneInputs {
        output: size,
        render: size,
        passes: false,
        hdr: false,
        surface_format: format,
        hdr_format,
    });
    let pipelines = pipelines::Pipelines::new(&device, plan.format, &shared);
    let targets = targets::SceneTargets::new(&device, &plan);
    let post = post::PostChain::new(&device);
    let instances = instances::InstanceBuffer::new(&device);
    let mut ui = super::ui::Ui::new(&device);
    ui.use_format(&device, format);
    let effects = super::effects::Effects::new(&device, plan.format, &shared.bindings);

    let api = api_of(adapter_info.backend);
    let info = BackendInfo {
        renderer: "blackbox",
        api,
        adapter: adapter_info.name.clone(),
        driver: format!("{} {}", adapter_info.driver, adapter_info.driver_info).trim().to_owned(),
    };
    let caps = native_capabilities(api, supports_bc, hdr_format == targets::HDR_FORMAT);

    let mut renderer = Renderer {
        output,
        device,
        queue,
        adapter_info,
        supports_bc,
        info,
        caps,
        graphics: GraphicsSettings::default(),
        captures: capture::Captures::default(),
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
    renderer
}
