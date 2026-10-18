//! C ABI to the SDK's C++ headers, pinned at build time to Streamline 2.14.16.
use super::super::temporal::TemporalFrame;
use ash::vk::{self, Handle};
use std::path::Path;

#[repr(C)]
pub(super) struct Image {
    image: u64,
    view: u64,
    width: u32,
    height: u32,
    format: u32,
    layout: u32,
    usage: u32,
}

#[repr(C)]
pub(super) struct Frame {
    projection: [f32; 16],
    inverse_projection: [f32; 16],
    reprojection: [f32; 16],
    inverse_reprojection: [f32; 16],
    camera_to_world: [f32; 16],
    jitter: [f32; 2],
    near: f32,
    fov: f32,
    delta_ms: f32,
    mode: u32,
    reset: u32,
}

unsafe extern "C" {
    #[cfg(test)]
    pub(super) fn bb_sl_evaluations() -> u64;
    fn bb_sl_init(directory: *const u16, logs: *const u16) -> i32;
    fn bb_sl_shutdown();
    fn bb_sl_release();
    fn bb_sl_supported(physical_device: *mut std::ffi::c_void) -> i32;
    fn bb_sl_size(mode: u32, width: u32, height: u32, size: *mut u32) -> i32;
    fn bb_sl_evaluate(command: *mut std::ffi::c_void, images: *const Image, frame: *const Frame) -> i32;
}

pub(super) fn release(device: &wgpu::Device) {
    if let Err(error) = device.poll(wgpu::PollType::wait_indefinitely()) {
        log::error!("DLSS resource drain failed: {error}");
        return;
    }
    // SAFETY: DlssPass is destroyed before Runtime, after its GPU submissions have completed.
    unsafe { bb_sl_release() };
}

fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let mut units: Vec<_> = path.as_os_str().encode_wide().collect();
    // The SDK appends ordinary separators internally; convert canonical verbatim paths to DOS/UNC form.
    if units.starts_with(&[92, 92, 63, 92, 85, 78, 67, 92]) {
        units.splice(..8, [92, 92]);
    }
    if units.starts_with(&[92, 92, 63, 92]) {
        units.drain(..4);
    }
    units.push(0);
    units
}

pub(super) fn initialize(directory: &Path, logs: &Path) -> Result<(), String> {
    let (directory, logs) = (wide(directory), wide(logs));
    // SAFETY: terminated UTF-16 strings live for the synchronous call; SL copies its preferences.
    let result = unsafe { bb_sl_init(directory.as_ptr(), logs.as_ptr()) };
    if result != 0 {
        return Err(format!("Streamline initialization failed ({result})"));
    }
    Ok(())
}

pub(super) fn shutdown() {
    // SAFETY: the Runtime owns the sole initialized session; its GPU has been drained before drop.
    unsafe { bb_sl_shutdown() };
}

pub(super) fn supported(physical_device: u64) -> bool {
    // SAFETY: this physical-device handle came from the live HAL device.
    unsafe { bb_sl_supported(physical_device as *mut _) == 0 }
}

pub(super) fn optimal_size(mode: u32, output: (u32, u32)) -> Option<(u32, u32)> {
    let mut size = [0; 2];
    // SAFETY: size is a writable two-element buffer; a supported runtime is active.
    let result = unsafe { bb_sl_size(mode, output.0, output.1, size.as_mut_ptr()) };
    (result == 0 && size[0] > 0 && size[1] > 0).then_some((size[0], size[1]))
}

pub(super) fn image(view: &wgpu::TextureView, output: bool) -> Option<Image> {
    let texture = view.texture();
    // SAFETY: both native handles are borrowed from a live wgpu image and never destroyed here.
    let image = {
        let hal = unsafe { texture.as_hal::<wgpu::hal::api::Vulkan>() }?;
        unsafe { hal.raw_handle() }.as_raw()
    };
    let native_view = {
        let hal = unsafe { view.as_hal::<wgpu::hal::api::Vulkan>() }?;
        unsafe { hal.raw_handle() }.as_raw()
    };
    let (format, layout, usage) = match (texture.format(), output) {
        (wgpu::TextureFormat::Depth32Float, _) => (
            vk::Format::D32_SFLOAT,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
        ),
        (wgpu::TextureFormat::R8Unorm, false) => (
            vk::Format::R8_UNORM,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
        ),
        (wgpu::TextureFormat::Rg16Float, _) => (
            vk::Format::R16G16_SFLOAT,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
        ),
        (wgpu::TextureFormat::Rgba16Float, true) => (
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageLayout::GENERAL,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
        ),
        (wgpu::TextureFormat::Rgba16Float, false) => (
            vk::Format::R16G16B16A16_SFLOAT,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
        ),
        (wgpu::TextureFormat::Bgra8Unorm, _) => (
            vk::Format::B8G8R8A8_UNORM,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
        ),
        (wgpu::TextureFormat::Rgb10a2Unorm, _) => (
            vk::Format::A2B10G10R10_UNORM_PACK32,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
        ),
        (wgpu::TextureFormat::Rgba8Unorm, _) => (
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
        ),
        _ => return None,
    };
    Some(Image {
        image,
        view: native_view,
        width: texture.width(),
        height: texture.height(),
        format: format.as_raw() as u32,
        layout: layout.as_raw() as u32,
        usage: usage.as_raw(),
    })
}

pub(super) fn evaluate(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    views: [&wgpu::TextureView; 4],
    frame: &TemporalFrame,
    mode: u32,
) -> Result<(), String> {
    let images = views
        .into_iter()
        .enumerate()
        .map(|(i, view)| image(view, i == 3))
        .collect::<Option<Vec<_>>>()
        .ok_or("DLSS needs Vulkan HDR/depth/motion images")?;
    encoder.transition_resources(
        std::iter::empty(),
        views.into_iter().enumerate().map(|(i, v)| wgpu::TextureTransition {
            texture: v.texture(),
            selector: None,
            state: match i {
                3 => wgpu::TextureUses::STORAGE_READ_WRITE,
                _ => wgpu::TextureUses::RESOURCE,
            },
        }),
    );
    let constants = frame_data(frame, mode);
    // SAFETY: resource transitions above match the tagged layouts. All resources outlive this encoder;
    // SL records only into its borrowed command buffer and restores resource layouts before returning.
    // wgpu 30 prohibits mixing its encoding API and raw commands on one encoder.
    // Submit the scene/transitions, a raw-only DLSS buffer, then continue on a fresh wgpu encoder.
    let continuation = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("after DLSS") });
    let before = std::mem::replace(encoder, continuation);
    queue.submit([before.finish()]);
    let mut native = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("native DLSS") });
    let result = unsafe {
        native.as_hal_mut::<wgpu::hal::api::Vulkan, _, _>(|hal| {
            let Some(hal) = hal else { return -1 };
            bb_sl_evaluate(hal.raw_handle().as_raw() as *mut _, images.as_ptr(), &constants)
        })
    };
    queue.submit([native.finish()]);
    // The raw buffer cannot register wgpu resources. Retain them until its submission completes,
    // including when a following resize or settings change replaces the scene and output images.
    let keepalive: Vec<_> = views.into_iter().cloned().collect();
    queue.on_submitted_work_done(move || drop(keepalive));
    if result != 0 {
        return Err(format!("slEvaluateFeature failed ({result})"));
    }
    Ok(())
}

pub(super) fn frame_data(frame: &TemporalFrame, mode: u32) -> Frame {
    let reprojection = frame.previous * frame.unjittered.inverse();
    let projection = frame.projection;
    // Column-vector glam storage equals row-major storage for SL's row-vector convention.
    Frame {
        projection: projection.to_cols_array(),
        inverse_projection: projection.inverse().to_cols_array(),
        camera_to_world: frame.camera_to_world.to_cols_array(),
        reprojection: reprojection.to_cols_array(),
        inverse_reprojection: reprojection.inverse().to_cols_array(),
        jitter: frame.jitter.to_array(),
        near: frame.near,
        fov: frame.fov_y,
        delta_ms: frame.delta_time * 1000.0,
        mode,
        reset: u32::from(frame.reset),
    }
}
