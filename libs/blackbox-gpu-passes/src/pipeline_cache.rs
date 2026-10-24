//! The Vulkan pipeline cache: the driver's compiled pipelines, kept in a file so the next start compiles fewer of
//! them. Every renderer built on these passes uses this code, so the file, its name and the way it is written are the
//! same whichever renderer made it. The other APIs have no such cache and get none.
//!
//! The file lives in the directory the caller names, under the key `wgpu::util::pipeline_cache_key` gives for the
//! adapter. It is derived data: delete it at any time.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// The file the cache for `info` lives in, under `dir`. `None` when the adapter has no cache key (only Vulkan has one).
pub fn file_for(dir: &Path, info: &wgpu::AdapterInfo) -> Option<PathBuf> {
    Some(dir.join(wgpu::util::pipeline_cache_key(info)?))
}

/// The cache for `device` and the file it is saved to. The cache starts from that file's data when there is one.
/// `None` when there is no `dir`, the adapter has no cache key or the device did not get `Features::PIPELINE_CACHE`.
pub fn open(
    device: &wgpu::Device,
    info: &wgpu::AdapterInfo,
    dir: Option<&Path>,
) -> Option<(wgpu::PipelineCache, PathBuf)> {
    let path = file_for(dir?, info)?;
    if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
        return None;
    }
    // A file that cannot be read starts the cache empty, like one that is not there.
    let data = std::fs::read(&path).ok();
    Some((create(device, data.as_deref()), path))
}

/// The cache for `device`, filled from `data` (what an earlier run saved), or empty without data.
#[allow(unsafe_code)]
fn create(device: &wgpu::Device, data: Option<&[u8]>) -> wgpu::PipelineCache {
    // SAFETY: `data` is only ever bytes a renderer saved with `PipelineCache::get_data` into the file `file_for` names,
    // in the per-user directory the caller gave (never shipped); nothing else is read here. `fallback: true` makes the
    // driver check the header (vendor, device and driver UUID) and start empty on a mismatch or corruption, so a stale
    // or damaged file costs only the compile time it would have saved.
    unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("blackbox pipelines"),
            data,
            fallback: true,
        })
    }
}

/// Write the cache's data to `path`. A cache without data (not Vulkan) writes nothing. Failures are logged, not
/// returned: a missing cache only costs compile time.
pub fn save(cache: &wgpu::PipelineCache, path: &Path) {
    let Some(data) = cache.get_data() else { return };
    write_logged(path, &data);
}

/// Like [`save`], but the data is only read here (the cache needs the device); the file is written on a thread of its
/// own, so a frame does not wait for the disk. Nothing waits for that thread: the write is atomic, and a process that
/// ends first keeps the previous file.
pub fn save_in_background(cache: &wgpu::PipelineCache, path: &Path) {
    let Some(data) = cache.get_data() else { return };
    let path = path.to_owned();
    let spawned =
        std::thread::Builder::new().name("pipeline cache save".to_owned()).spawn(move || write_logged(&path, &data));
    if let Err(e) = spawned {
        log::warn!("could not start the pipeline cache save: {e}");
    }
}

fn write_logged(path: &Path, data: &[u8]) {
    match write_atomic(path, data) {
        Ok(()) => log::debug!("saved the pipeline cache to {} ({} bytes)", path.display(), data.len()),
        Err(e) => log::warn!("could not save the pipeline cache to {}: {e}", path.display()),
    }
}

/// Write `data` to `path` through a `.tmp` file renamed over it, so a crash mid-write keeps the previous file. The
/// temporary file is named per write, so two saves running at once do not share one.
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    static WRITES: AtomicU32 = AtomicU32::new(0);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(format!(".{}.tmp", WRITES.fetch_add(1, Ordering::Relaxed)));
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, data)?;
    std::fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(backend: wgpu::Backend) -> wgpu::AdapterInfo {
        wgpu::AdapterInfo {
            name: "test adapter".to_owned(),
            vendor: 0x10de,
            device: 0x2684,
            device_type: wgpu::DeviceType::DiscreteGpu,
            device_pci_bus_id: String::new(),
            driver: String::new(),
            driver_info: String::new(),
            backend,
            subgroup_min_size: 0,
            subgroup_max_size: 0,
            transient_saves_memory: None,
            limit_bucket: None,
        }
    }

    #[test]
    fn a_vulkan_adapter_saves_to_a_file_named_after_its_vendor_and_device() {
        let dir = Path::new("cache").join("pipelines");
        let path = file_for(&dir, &info(wgpu::Backend::Vulkan)).expect("Vulkan has a cache key");
        // 0x10de and 0x2684 in decimal.
        assert_eq!(path, dir.join("wgpu_pipeline_cache_vulkan_4318_9860"));
    }

    #[test]
    fn an_adapter_without_a_cache_key_has_no_file() {
        assert_eq!(file_for(Path::new("cache"), &info(wgpu::Backend::Dx12)), None);
        assert_eq!(file_for(Path::new("cache"), &info(wgpu::Backend::Gl)), None);
    }

    #[test]
    fn writing_replaces_the_file_and_leaves_no_temporary_one() {
        let dir = std::env::temp_dir().join(format!("blackbox-gpu-passes-pipeline-cache-{}", std::process::id()));
        let path = dir.join("cache");
        write_atomic(&path, b"first").expect("a first write");
        write_atomic(&path, b"second").expect("a second write");
        assert_eq!(std::fs::read(&path).expect("read back"), b"second".to_vec());
        let leftovers = std::fs::read_dir(&dir).expect("list the directory").count();
        assert_eq!(leftovers, 1, "only the cache file is left");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
