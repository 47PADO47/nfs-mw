//! Optional Windows Vulkan/DX12 Streamline integration. Native handles never escape this module.
#![allow(unsafe_code, reason = "Streamline and wgpu HAL expose native graphics handles through FFI")]

mod composition;
#[cfg(test)]
mod composition_tests;
pub(super) mod dx12;
mod native;
pub(super) mod pass;
pub(super) mod presentation;
#[cfg(test)]
mod presentation_tests;
#[cfg(test)]
mod test_window;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ash::vk;
use blackbox_gfx::UpscaleQuality;

static IN_USE: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);

pub(super) struct Runtime {
    pub supported: bool,
    pub failed: Arc<AtomicBool>,
}

impl Runtime {
    /// This integration permits one Streamline device session per process.
    pub(super) fn create(desc: &mut wgpu::InstanceDescriptor, directx: bool) -> Result<(wgpu::Instance, Self), String> {
        if STARTED.load(Ordering::Acquire) {
            return Err(
                "this integration permits one Streamline device session per process; restart to enable DLSS again"
                    .into(),
            );
        }
        if IN_USE.swap(true, Ordering::AcqRel) {
            return Err("another renderer owns Streamline".into());
        }
        let runtime = Self { supported: false, failed: Arc::default() };
        let directory = std::env::var_os("STREAMLINE_PLUGIN_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok()?.parent().map(PathBuf::from))
            .ok_or("cannot locate Streamline plugins")?;
        let directory = directory.canonicalize().map_err(|e| e.to_string())?;
        let logs = std::env::temp_dir().join("blackbox-streamline");
        std::fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
        native::initialize(&directory, &logs, directx)?;
        STARTED.store(true, Ordering::Release);
        if directx {
            return Ok((
                wgpu::Instance::new(std::mem::replace(desc, wgpu::InstanceDescriptor::new_without_display_handle())),
                runtime,
            ));
        }
        // The interposer proxies vkCreateInstance, vkCreateDevice and all presentation hooks.
        // Its library remains resident until process exit, including after slShutdown.
        let entry = unsafe { ash::Entry::load_from(directory.join("sl.interposer.dll")) }.map_err(|e| e.to_string())?;
        let version = vk::API_VERSION_1_3;
        let flags = wgpu::InstanceFlags::empty();
        let extensions =
            wgpu::hal::vulkan::Instance::desired_extensions(&entry, version, flags).map_err(|e| e.to_string())?;
        let names: Vec<_> = extensions.iter().map(|name| name.as_ptr()).collect();
        let app = vk::ApplicationInfo::default().api_version(version);
        let info = vk::InstanceCreateInfo::default().application_info(&app).enabled_extension_names(&names);
        // SAFETY: the loader and enabled extensions remain valid for the owned HAL instance.
        let raw = unsafe { entry.create_instance(&info, None) }.map_err(|e| e.to_string())?;
        let hal = unsafe {
            wgpu::hal::vulkan::Instance::from_raw(
                entry,
                raw,
                version,
                0,
                None,
                extensions,
                flags,
                wgpu::MemoryBudgetThresholds::default(),
                false,
                None,
            )
        }
        .map_err(|e| e.to_string())?;
        Ok((unsafe { wgpu::Instance::from_hal::<wgpu::hal::api::Vulkan>(hal) }, runtime))
    }

    pub(super) fn check(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.supported {
            return;
        }
        if unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }.is_some() {
            self.supported = dx12::attach(device, queue) && native::supported(0);
            return;
        }
        // SAFETY: the borrowed device is live and belongs to this Streamline loader.
        let Some(device) = (unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }) else { return };
        use ash::vk::Handle;
        self.supported = native::supported(device.raw_physical_device().as_raw());
        if !self.supported {
            log::warn!("Streamline DLSS is unavailable on this adapter; using normal upscaler fallback");
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        native::shutdown();
        IN_USE.store(false, Ordering::Release);
    }
}

pub(super) fn mode(quality: UpscaleQuality) -> u32 {
    match quality {
        UpscaleQuality::Native => 6,
        UpscaleQuality::Performance => 1,
        UpscaleQuality::Balanced => 2,
        UpscaleQuality::UltraPerformance => 4,
        _ => 3,
    }
}

pub(super) fn render_size(quality: UpscaleQuality, output: (u32, u32), model: u32) -> Option<(u32, u32)> {
    native::optimal_size(mode(quality), output, model)
}
