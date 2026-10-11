//! Which graphics APIs this PC has a GPU for, so the settings offer only those.

use blackbox_gfx::GraphicsApi;

/// `Auto` first, then each of Vulkan, Direct3D 12 and OpenGL that has a GPU adapter. Each API is asked for on its
/// own, so a missing driver for one does not hide the others.
pub fn available_apis() -> Vec<GraphicsApi> {
    let explicit = [GraphicsApi::Vulkan, GraphicsApi::Dx12, GraphicsApi::Gl];
    std::iter::once(GraphicsApi::Auto).chain(explicit.into_iter().filter(|api| has_gpu(*api))).collect()
}

/// Whether wgpu finds a hardware adapter for `api`. Software adapters (WARP, lavapipe) do not count: they would
/// offer an API that runs, but far too slowly to play on.
fn has_gpu(api: GraphicsApi) -> bool {
    let backends = super::init::wgpu_backends(api);
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    pollster::block_on(instance.enumerate_adapters(backends))
        .iter()
        .any(|adapter| adapter.get_info().device_type != wgpu::DeviceType::Cpu)
}
