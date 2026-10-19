//! D3D12 native command submission and proxy DXGI presentation, without mutating wgpu internals.
use super::native::{Frame, Image};
use windows::core::Interface;

unsafe extern "C" {
    fn bb_sl_dx12_depth(device: *mut std::ffi::c_void, width: u32, height: u32) -> *mut std::ffi::c_void;
    fn bb_sl_dx12_attach(device: *mut std::ffi::c_void, queue: *mut std::ffi::c_void) -> i32;
    fn bb_sl_dx12_configure(
        window: *mut std::ffi::c_void,
        width: u32,
        height: u32,
        format: u32,
        color_space: u32,
    ) -> i32;
    fn bb_sl_dx12_evaluate(images: *const Image, frame: *const Frame) -> i32;
    fn bb_sl_dx12_present(source: *mut std::ffi::c_void, vsync: u32) -> i32;
}
pub(super) fn attach(device: &wgpu::Device, queue: &wgpu::Queue) -> bool {
    let Some(hal) = (unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }) else { return false };
    let Some(queue) = (unsafe { queue.as_hal::<wgpu::hal::api::Dx12>() }) else { return false };
    let result = unsafe { bb_sl_dx12_attach(hal.raw_device().as_raw(), queue.as_raw().as_raw()) };
    if result != 0 {
        log::warn!("Streamline D3D12 attachment failed ({result})");
    }
    result == 0
}
pub(super) fn format(format: wgpu::TextureFormat) -> Option<u32> {
    // DXGI_FORMAT values. Streamline uses native D3D12_RESOURCE_STATES, not Vulkan layouts.
    match format {
        wgpu::TextureFormat::Rgba16Float => Some(10),
        wgpu::TextureFormat::Rgb10a2Unorm => Some(24),
        wgpu::TextureFormat::Rgba8Unorm => Some(28),
        wgpu::TextureFormat::Rg16Float => Some(34),
        wgpu::TextureFormat::Depth32Float => Some(39),
        wgpu::TextureFormat::R16Float => Some(54),
        wgpu::TextureFormat::Bgra8Unorm => Some(87),
        _ => None,
    }
}
pub(super) fn image(view: &wgpu::TextureView, output: bool) -> Option<Image> {
    let texture = view.texture();
    let hal = unsafe { texture.as_hal::<wgpu::hal::api::Dx12>() }?;
    Some(Image {
        image: unsafe { hal.raw_resource() }.as_raw() as u64,
        view: 0,
        width: texture.width(),
        height: texture.height(),
        format: format(texture.format())?,
        layout: match output {
            true => 8,
            false => 192,
        },
        usage: 0,
    })
}
pub(super) fn evaluate(images: &[Image], frame: &Frame) -> i32 {
    unsafe { bb_sl_dx12_evaluate(images.as_ptr(), frame) }
}

/// The wgpu surface is retained for monitor/HDR metadata and window lifetime only. Its swapchain
/// is never configured. A native proxy swapchain presents a copy of this wgpu-owned target.
pub(in crate::gpu) struct Surface {
    target: wgpu::Texture,
    window: isize,
    error: Option<String>,
}
impl Surface {
    pub(in crate::gpu) fn new(
        window: isize,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Result<Self, String> {
        let mut result = Self { target: target(device, config), window, error: None };
        result.configure(device, config);
        if let Some(error) = result.error.take() {
            return Err(error);
        }
        Ok(result)
    }
    pub(in crate::gpu) fn configure(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let color_space = match config.color_space {
            wgpu::SurfaceColorSpace::Bt2100Pq => 12,
            wgpu::SurfaceColorSpace::ExtendedSrgbLinear => 1,
            _ => 0,
        };
        let result = unsafe {
            bb_sl_dx12_configure(
                self.window as *mut _,
                config.width,
                config.height,
                format(config.format).unwrap_or(0),
                color_space,
            )
        };
        self.error = (result < 0).then(|| format!("Streamline DXGI configure failed ({result})"));
        if self.error.is_none() {
            self.target = target(device, config);
        }
    }
    pub(in crate::gpu) fn view(&self) -> Result<wgpu::TextureView, crate::RenderError> {
        if let Some(error) = &self.error {
            return Err(crate::RenderError::Surface(error.clone()));
        }
        Ok(self.target.create_view(&Default::default()))
    }
    pub(in crate::gpu) fn present(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vsync: bool,
    ) -> Result<(), crate::RenderError> {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("DXGI copy source") });
        encoder.transition_resources(
            std::iter::empty(),
            [wgpu::TextureTransition { texture: &self.target, selector: None, state: wgpu::TextureUses::COPY_SRC }]
                .into_iter(),
        );
        queue.submit([encoder.finish()]);
        let hal = unsafe { self.target.as_hal::<wgpu::hal::api::Dx12>() }.expect("DX12 target");
        let result = unsafe { bb_sl_dx12_present(hal.raw_resource().as_raw(), u32::from(vsync)) };
        if result < 0 {
            return Err(crate::RenderError::Surface(format!("Streamline DXGI present failed ({result})")));
        }
        Ok(())
    }
}
fn target(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("DX12 final frame before Streamline presentation"),
        size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Typed D32 resources are legal on modern D3D12, but SL 2.14.16's format table omits D32_FLOAT.
/// Use the standard R32 typeless depth allocation, with wgpu still owning its typed views/barriers.
pub(in crate::gpu) fn depth(device: &wgpu::Device, width: u32, height: u32) -> Option<wgpu::TextureView> {
    let native = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }?;
    let raw = unsafe { bb_sl_dx12_depth(native.raw_device().as_raw(), width.max(1), height.max(1)) };
    if raw.is_null() {
        return None;
    }
    let size = wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 };
    let desc = wgpu::TextureDescriptor {
        label: Some("Streamline-compatible typeless depth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    // Owned reference returned by CreateCommittedResource; the default heap is zero-initialized.
    // The first scene pass clears all depth again. COMMON maps to wgpu's UNINITIALIZED state.
    let resource = unsafe { windows::Win32::Graphics::Direct3D12::ID3D12Resource::from_raw(raw) };
    let texture = unsafe {
        let hal = wgpu::hal::dx12::Device::texture_from_raw(resource, desc.format, desc.dimension, size, 1, 1);
        device.create_texture_from_hal::<wgpu::hal::api::Dx12>(hal, &desc, wgpu::TextureUses::UNINITIALIZED)
    };
    Some(texture.create_view(&Default::default()))
}
