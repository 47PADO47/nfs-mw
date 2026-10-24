//! When the native renderer saves its Vulkan pipeline cache. The cache, its file and the way the file is written are
//! shared with the other renderers built on the same passes (`blackbox_gpu_passes::pipeline_cache`); this module only
//! decides when to save.
//!
//! The file lives in the directory `RendererOptions::pipeline_cache_dir` names. It is derived data: delete it at any
//! time.

use super::Renderer;

/// Frames after which the cache is saved. By then the pipelines of the first scenes exist, and a later crash
/// still keeps them.
const WARM_UP_FRAMES: u32 = 300;

impl Renderer {
    /// Save the pipeline cache to its file now. Does nothing without a cache, which needs a
    /// `pipeline_cache_dir` in the options and the Vulkan backend. The renderer also saves it after its first
    /// frames and when it is dropped.
    pub fn save_pipeline_cache(&self) {
        let (Some(cache), Some(path)) = (&self.pipeline_cache, &self.pipeline_cache_path) else {
            return;
        };
        blackbox_gpu_passes::pipeline_cache::save(cache, path);
    }

    /// Count a drawn frame, and save the pipeline cache once the first frames have built their pipelines. The file is written off the frame
    /// thread.
    pub(super) fn count_frame(&mut self) {
        self.frames = self.frames.saturating_add(1);
        if self.frames != WARM_UP_FRAMES {
            return;
        }
        let (Some(cache), Some(path)) = (&self.pipeline_cache, &self.pipeline_cache_path) else {
            return;
        };
        blackbox_gpu_passes::pipeline_cache::save_in_background(cache, path);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.save_pipeline_cache();
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            self.presentation.suspend(&self.device);
            self.post.set_temporal(None);
            self.dlss.take();
        }
    }
}
