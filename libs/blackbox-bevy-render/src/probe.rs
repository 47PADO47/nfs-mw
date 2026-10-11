//! The adapter probe: what the machine can run, found out before the Bevy `App` is built.
//!
//! Bevy panics when it cannot find an adapter after the app exists, so every fallback to another renderer
//! has to happen first. [`probe`] opens a throw-away wgpu instance, asks for the adapter Bevy would pick and
//! reports what matters for this renderer.

use blackbox_gfx::GraphicsApi;

/// Who made the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

impl Vendor {
    /// From a PCI vendor id (Vulkan and Direct3D 12 report one).
    pub fn from_pci(id: u32) -> Self {
        match id {
            0x10de => Self::Nvidia,
            0x1002 | 0x1022 => Self::Amd,
            0x8086 | 0x8087 => Self::Intel,
            _ => Self::Other,
        }
    }
}

/// What the adapter offers, as far as this renderer cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The API of the adapter that was found (never [`GraphicsApi::Auto`]).
    pub api: GraphicsApi,
    pub adapter: String,
    pub driver: String,
    pub vendor: Vendor,
    /// A CPU rasteriser (lavapipe, WARP).
    pub software: bool,
    /// BC1 to BC3 textures upload without decoding.
    pub compressed_bc: bool,
    /// Hardware ray queries (the ray tracing feature of later PRs).
    pub ray_query: bool,
    /// Texture binding arrays with non-uniform indexing (bindless materials).
    pub binding_arrays: bool,
}

/// Why this renderer cannot run here.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProbeError {
    /// The Bevy renderer does not support this API.
    #[error("the bevy renderer does not support {0}")]
    Unsupported(GraphicsApi),
    /// No adapter for the API.
    #[error("no GPU adapter: {0}")]
    NoAdapter(String),
}

/// The wgpu backends that stand for `api`, or `None` when the Bevy renderer cannot use it.
pub fn backends_for(api: GraphicsApi) -> Option<wgpu::Backends> {
    match api {
        GraphicsApi::Auto => Some(wgpu::Backends::VULKAN | wgpu::Backends::DX12 | wgpu::Backends::METAL),
        GraphicsApi::Vulkan => Some(wgpu::Backends::VULKAN),
        GraphicsApi::Dx12 => Some(wgpu::Backends::DX12),
        GraphicsApi::Gl => None,
    }
}

/// The API a wgpu backend stands for.
pub fn api_of(backend: wgpu::Backend) -> GraphicsApi {
    match backend {
        wgpu::Backend::Vulkan => GraphicsApi::Vulkan,
        wgpu::Backend::Dx12 => GraphicsApi::Dx12,
        wgpu::Backend::Gl => GraphicsApi::Gl,
        _ => GraphicsApi::Auto,
    }
}

/// Look for an adapter for `api` and report its capabilities. `force_fallback` asks for the API's software
/// adapter (lavapipe, WARP), like `WGPU_FORCE_FALLBACK_ADAPTER` does for Bevy itself.
pub fn probe_with(api: GraphicsApi, force_fallback: bool) -> Result<Probe, ProbeError> {
    let Some(backends) = backends_for(api) else { return Err(ProbeError::Unsupported(api)) };
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    let request = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: force_fallback,
        compatible_surface: None,
        apply_limit_buckets: false,
    };
    let adapter = pollster::block_on(instance.request_adapter(&request))
        .map_err(|e| ProbeError::NoAdapter(format!("{api}: {e}")))?;
    let info = adapter.get_info();
    let features = adapter.features();
    let arrays = wgpu::Features::TEXTURE_BINDING_ARRAY
        | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;
    Ok(Probe {
        api: api_of(info.backend),
        adapter: info.name.clone(),
        driver: format!("{} {}", info.driver, info.driver_info).trim().to_owned(),
        vendor: Vendor::from_pci(info.vendor),
        software: info.device_type == wgpu::DeviceType::Cpu,
        compressed_bc: features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        ray_query: features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY),
        binding_arrays: features.contains(arrays),
    })
}

/// [`probe_with`], honouring `BLACKBOX_GPU_FALLBACK` / `WGPU_FORCE_FALLBACK_ADAPTER` (any value but empty, `0`
/// or `false`).
pub fn probe(api: GraphicsApi) -> Result<Probe, ProbeError> {
    let set = |name: &str| std::env::var(name).is_ok_and(|v| !(v.is_empty() || v == "0" || v == "false"));
    probe_with(api, set("BLACKBOX_GPU_FALLBACK") || set("WGPU_FORCE_FALLBACK_ADAPTER"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendors_come_from_pci_ids() {
        assert_eq!(Vendor::from_pci(0x10de), Vendor::Nvidia);
        assert_eq!(Vendor::from_pci(0x1002), Vendor::Amd);
        assert_eq!(Vendor::from_pci(0x8086), Vendor::Intel);
        assert_eq!(Vendor::from_pci(0x1234), Vendor::Other);
    }

    #[test]
    fn opengl_is_not_supported_and_the_rest_map_to_backends() {
        assert_eq!(backends_for(GraphicsApi::Gl), None);
        assert_eq!(backends_for(GraphicsApi::Vulkan), Some(wgpu::Backends::VULKAN));
        assert_eq!(backends_for(GraphicsApi::Dx12), Some(wgpu::Backends::DX12));
        assert!(backends_for(GraphicsApi::Auto).is_some());
    }

    #[test]
    fn probing_opengl_fails_without_touching_the_gpu() {
        assert_eq!(probe_with(GraphicsApi::Gl, false), Err(ProbeError::Unsupported(GraphicsApi::Gl)));
    }

    #[test]
    fn backends_map_back_to_apis() {
        assert_eq!(api_of(wgpu::Backend::Vulkan), GraphicsApi::Vulkan);
        assert_eq!(api_of(wgpu::Backend::Dx12), GraphicsApi::Dx12);
        assert_eq!(api_of(wgpu::Backend::Gl), GraphicsApi::Gl);
    }

    #[test]
    fn probing_this_machine_reports_an_adapter_or_says_why_not() {
        let _gpu = blackbox_gpu_passes::test_support::serial();
        match probe(GraphicsApi::Auto) {
            Ok(found) => {
                eprintln!("{found:?}");
                assert_ne!(found.api, GraphicsApi::Auto);
                assert!(!found.adapter.is_empty());
            }
            Err(e) => eprintln!("skipping: {e}"),
        }
    }
}
