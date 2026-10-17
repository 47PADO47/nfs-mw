//! Instance, surface, adapter and device setup.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use super::output::Output;
use super::{Renderer, capture, instances, pipelines, post, resources, slots::Slots, targets};
use crate::caps::native_capabilities;
use crate::{Backend, DEFAULT_RENDER_SCALE, PixelFormat, RenderError, RendererOptions, TextureDesc};
use blackbox_gfx::{BackendInfo, GraphicsApi, GraphicsSettings};

pub(super) fn wgpu_backends(backend: Backend) -> wgpu::Backends {
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
    let mut required_features =
        if supports_bc { wgpu::Features::TEXTURE_COMPRESSION_BC } else { wgpu::Features::empty() };
    if adapter.features().contains(super::timing::FEATURES) {
        required_features |= super::timing::FEATURES;
    }
    let mut limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
    if adapter.get_downlevel_capabilities().flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS) {
        // The compute limits the optional AO pass and the FSR 3 upscaler need, as far as the adapter has them.
        // Retains the GL fallback, which asks for what it has and so reports no FSR 3.
        let have = adapter.limits();
        let want = temporal_limits();
        limits.max_compute_workgroup_size_x = have.max_compute_workgroup_size_x.min(want.max_compute_workgroup_size_x);
        limits.max_compute_workgroup_size_y = have.max_compute_workgroup_size_y.min(want.max_compute_workgroup_size_y);
        limits.max_compute_workgroup_size_z = have.max_compute_workgroup_size_z.min(want.max_compute_workgroup_size_z);
        limits.max_compute_invocations_per_workgroup =
            have.max_compute_invocations_per_workgroup.min(want.max_compute_invocations_per_workgroup);
        limits.max_compute_workgroups_per_dimension = 65535;
        limits.max_storage_textures_per_shader_stage =
            have.max_storage_textures_per_shader_stage.min(want.max_storage_textures_per_shader_stage);
        limits.max_storage_buffers_per_shader_stage =
            have.max_storage_buffers_per_shader_stage.min(want.max_storage_buffers_per_shader_stage);
        limits.max_storage_buffer_binding_size =
            have.max_storage_buffer_binding_size.min(want.max_storage_buffer_binding_size);
    }
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("blackbox-render device"),
        required_features,
        required_limits: limits,
        ..Default::default()
    }))
    .map_err(|e| RenderError::Device(e.to_string()))
}

/// The compute limits FSR 3 runs within: wgpu's defaults for storage bindings and a 256-thread workgroup (the
/// luma pyramid's reduction). The AO pass needs less of the same.
fn temporal_limits() -> wgpu::Limits {
    let mut limits = wgpu::Limits::downlevel_webgl2_defaults();
    limits.max_compute_workgroup_size_x = 256;
    limits.max_compute_workgroup_size_y = 8;
    limits.max_compute_workgroup_size_z = 1;
    limits.max_compute_invocations_per_workgroup = 256;
    limits.max_storage_textures_per_shader_stage = 4;
    limits.max_storage_buffers_per_shader_stage = 8;
    limits.max_storage_buffer_binding_size = wgpu::Limits::default().max_storage_buffer_binding_size;
    limits
}

/// Whether a device with `limits` can run the FSR 3 upscaler: its passes bind four storage textures and eight
/// storage buffers, and reduce in a workgroup of 256 threads.
fn supports_temporal(limits: &wgpu::Limits) -> bool {
    let need = temporal_limits();
    limits.max_storage_textures_per_shader_stage >= need.max_storage_textures_per_shader_stage
        && limits.max_storage_buffers_per_shader_stage >= need.max_storage_buffers_per_shader_stage
        && limits.max_compute_workgroup_size_x >= need.max_compute_workgroup_size_x
        && limits.max_compute_workgroup_size_y >= need.max_compute_workgroup_size_y
        && limits.max_compute_invocations_per_workgroup >= need.max_compute_invocations_per_workgroup
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
        temporal: false,
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
    let caps = native_capabilities(
        api,
        supports_bc,
        hdr_format == targets::HDR_FORMAT,
        device.limits().max_compute_invocations_per_workgroup >= 64,
        // The scene is drawn in HDR, the upscaler writes Rgba16Float storage, and Vulkan or DirectX runs it.
        api != GraphicsApi::Gl && hdr_format == targets::HDR_FORMAT && supports_temporal(&device.limits()),
    );

    let timing = super::timing::Timing::new(&device, &queue);
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
        view_capture: None,
        targets,
        hdr_format,
        post,
        reflections: None,
        experiments: Default::default(),
        experiment_stats: blackbox_gfx::ExperimentStats { supported: api != GraphicsApi::Gl, ..Default::default() },
        shadows: None,
        smaa: None,
        shadow_casters: None,
        ao: None,
        contact: None,
        surfaces: None,
        reuse_prepass_depth: true,
        timing,
        render_scale: DEFAULT_RENDER_SCALE,
        upscale: Default::default(),
        temporal: Default::default(),
        shared,
        pipelines,
        textures: Slots::new(),
        meshes: Slots::new(),
        instances,
        effects,
        glossy: None,
        ui,
        redirects: Default::default(),
        draw_list: Default::default(),
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
