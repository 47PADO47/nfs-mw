//! What a backend reports about itself.

use crate::GraphicsApi;

/// Which renderer and GPU is drawing. Fixed once the backend exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendInfo {
    /// "blackbox", "bevy"...
    pub renderer: &'static str,
    pub api: GraphicsApi,
    /// The GPU's name.
    pub adapter: String,
    /// The driver's name and version, when the API reports one.
    pub driver: String,
}

impl BackendInfo {
    /// "GPU name (api)" for logs and the window title.
    pub fn summary(&self) -> String {
        format!("{} ({})", self.adapter, self.api)
    }
}

/// Live counts for stats overlays and leak checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub meshes: usize,
    pub textures: usize,
    /// Allocated capacities in vertices of the surface and particle effect batches.
    pub effect_capacities: [usize; 2],
    /// Allocated additive-streak capacity in vertices; retained when the streak layer is cleared.
    pub streak_capacity: usize,
}

/// Main-thread wall timings and asynchronous GPU timestamps for the last rendered frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct FramePerformance {
    pub rendered_frames: u64,
    /// OS activity over a recent sampling interval, independent of generated presentation counts.
    pub activity: WorkloadActivity,
    /// Command recording/submission, excluding explicit surface acquisition and presentation waits.
    pub cpu_render_ms: f32,
    pub acquire_wait_ms: f32,
    pub present_wait_ms: f32,
    /// Render queue span; includes SR/post processing, excludes present-time frame generation.
    pub gpu_render_ms: Option<f32>,
    /// Advances when a new asynchronous timestamp readback completes.
    pub gpu_sample: u64,
}

/// OS scheduler activity. Missing/stale counters stay unknown rather than becoming zero utilization.
/// GPU activity is this process's busiest engine across adapters, including SDK/auxiliary queues.
/// Main-thread activity excludes time asleep/blocked, but includes driver CPU work on that thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkloadActivity {
    pub main_thread_busy_percent: Option<f32>,
    pub gpu_busy_percent: Option<f32>,
    pub gpu_engine: &'static str,
    pub sample: u64,
    pub age_ms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_names_the_gpu_and_the_api() {
        let info = BackendInfo {
            renderer: "blackbox",
            api: GraphicsApi::Vulkan,
            adapter: "Intel Iris Xe".into(),
            driver: String::new(),
        };
        assert_eq!(info.summary(), "Intel Iris Xe (vulkan)");
    }

    #[test]
    fn stats_start_at_zero() {
        assert_eq!(
            RenderStats::default(),
            RenderStats { meshes: 0, textures: 0, effect_capacities: [0; 2], streak_capacity: 0 }
        );
    }
}
