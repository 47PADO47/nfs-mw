//! The Vulkan pipeline cache: the driver's compiled pipelines, kept in a file so the next start compiles fewer of
//! them. Every pipeline the renderer builds takes it; the other APIs have no such cache and get none.
//!
//! The file lives in the directory `RendererOptions::pipeline_cache_dir` names, under the key
//! `wgpu::util::pipeline_cache_key` gives for the adapter. It is derived data: delete it at any time.

use std::path::{Path, PathBuf};

use super::Renderer;

/// Frames after which the cache is saved. By then the pipelines of the first scenes exist, and a later crash
/// still keeps them.
const WARM_UP_FRAMES: u32 = 300;

/// The cache for the device `info` describes, and the file it is saved to. `None` when there is no directory, the
/// adapter has no cache key (only Vulkan has one) or the device did not get `Features::PIPELINE_CACHE`.
pub(super) fn open(
    device: &wgpu::Device,
    info: &wgpu::AdapterInfo,
    dir: Option<&Path>,
) -> Option<(wgpu::PipelineCache, PathBuf)> {
    let dir = dir?;
    if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
        return None;
    }
    let path = dir.join(wgpu::util::pipeline_cache_key(info)?);
    // A file that cannot be read starts the cache empty, like one that is not there.
    let data = std::fs::read(&path).ok();
    Some((create(device, data.as_deref()), path))
}

/// The cache for `device`, filled from `data` (what an earlier run saved), or empty without data.
#[allow(unsafe_code)]
fn create(device: &wgpu::Device, data: Option<&[u8]>) -> wgpu::PipelineCache {
    // SAFETY: `data` is only ever bytes this renderer saved with `PipelineCache::get_data` into its own per-user
    // file, named after the adapter (see `open`); nothing else is read here. `fallback: true` makes the driver check
    // the header (vendor, device and driver UUID) and start empty on a mismatch or corruption, so a stale or damaged
    // file costs only the compile time it would have saved.
    unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("blackbox pipelines"),
            data,
            fallback: true,
        })
    }
}

/// Write the cache's data to `path`. The data goes to a temporary file that is then renamed over `path`, so a
/// crash mid-write keeps the previous cache.
pub(super) fn save(cache: &wgpu::PipelineCache, path: &Path) {
    let data = match cache.get_data() {
        Some(data) => data,
        None => return,
    };
    match write_atomic(path, &data) {
        Ok(()) => log::debug!("saved the pipeline cache to {} ({} bytes)", path.display(), data.len()),
        Err(e) => log::warn!("could not save the pipeline cache to {}: {e}", path.display()),
    }
}

fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, data)?;
    std::fs::rename(&temp, path)
}

impl Renderer {
    /// Save the pipeline cache to its file now. Does nothing without a cache, which needs a
    /// `pipeline_cache_dir` in the options and the Vulkan backend. The renderer also saves it after its first
    /// frames and when it is dropped.
    pub fn save_pipeline_cache(&self) {
        let (Some(cache), Some(path)) = (&self.pipeline_cache, &self.pipeline_cache_path) else {
            return;
        };
        save(cache, path);
    }

    /// Count a drawn frame, and save the pipeline cache once the first frames have built their pipelines.
    pub(super) fn count_frame(&mut self) {
        self.frames = self.frames.saturating_add(1);
        if self.frames == WARM_UP_FRAMES {
            self.save_pipeline_cache();
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.save_pipeline_cache();
    }
}
