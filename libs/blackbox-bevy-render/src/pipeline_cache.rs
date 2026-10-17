//! The Vulkan pipeline cache for the passes this backend builds: the same file the native renderer keeps, written
//! by the same code (`blackbox_gpu_passes::pipeline_cache`). Bevy's own pipelines are not in it: bevy_render 0.20
//! creates them with `cache: None`, which this backend cannot change.
//!
//! The resource lives in the render world. The plugin inserts it with the directory it was given, `RenderStartup`
//! opens the file once the device exists, the passes take [`PipelineCacheFile::cache`] when they are built, and the
//! render world saves it after the warm-up frames and when it is dropped at exit.

use std::path::PathBuf;

use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Res, ResMut};
use bevy_render::renderer::{RenderAdapterInfo, RenderDevice};

/// Frames after which the cache is saved, as in the native renderer: by then the first scenes' pipelines exist.
const WARM_UP_FRAMES: u32 = 300;

/// The pipeline cache of the render world and the file it is saved to. It has no cache when the plugin was given no
/// directory, the adapter has no cache key (only Vulkan has one) or the device lacks `Features::PIPELINE_CACHE`.
#[derive(Resource, Default)]
pub struct PipelineCacheFile {
    /// The directory the plugin was given (`BlackboxBevyRenderPlugin::pipeline_cache_dir`).
    dir: Option<PathBuf>,
    cache: Option<wgpu::PipelineCache>,
    path: Option<PathBuf>,
    /// Frames drawn so far.
    frames: u32,
}

impl PipelineCacheFile {
    /// The resource for `dir` (`None` keeps no cache). The file is opened once the device exists, by [`open`].
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self { dir, cache: None, path: None, frames: 0 }
    }

    /// The cache every pass this backend builds takes, when there is one.
    pub fn cache(&self) -> Option<&wgpu::PipelineCache> {
        self.cache.as_ref()
    }

    /// Save the cache to its file now. Does nothing without a cache.
    fn save(&self) {
        let (Some(cache), Some(path)) = (&self.cache, &self.path) else {
            return;
        };
        blackbox_gpu_passes::pipeline_cache::save(cache, path);
    }
}

/// Saves the cache when the render world goes away, which is how it is kept at exit.
impl Drop for PipelineCacheFile {
    fn drop(&mut self) {
        self.save();
    }
}

/// Opens the cache for the device. Bevy runs `RenderStartup` once the device exists, and again after a lost device is
/// recreated, so each device gets a cache of its own.
pub fn open(mut file: ResMut<PipelineCacheFile>, device: Res<RenderDevice>, info: Res<RenderAdapterInfo>) {
    let opened = blackbox_gpu_passes::pipeline_cache::open(device.wgpu_device(), &info, file.dir.as_deref());
    let Some((cache, path)) = opened else { return };
    file.cache = Some(cache);
    file.path = Some(path);
}

/// Counts the frames drawn and saves the cache once the first ones have built their pipelines. Runs after the render
/// graph (`RenderSystems::Cleanup`), so the passes of those frames exist by then.
pub fn count_frame(mut file: ResMut<PipelineCacheFile>) {
    file.frames = file.frames.saturating_add(1);
    if file.frames == WARM_UP_FRAMES {
        file.save();
    }
}
