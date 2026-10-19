//! Frame statistics, as plain data. The overlay draws them; nothing here knows about egui.

mod rates;
mod work;

use std::collections::VecDeque;
use std::fmt;
use std::str::FromStr;
use std::time::Instant;

use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};

use super::ShowReadout;
use crate::app::Host;

/// How much the performance overlay shows (`--show-metrics`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowMetrics {
    #[default]
    Off,
    /// Frame rate and frame time.
    Basic,
    /// Plus 1% lows, a frame-time graph, GPU and resource counts.
    Advanced,
}

impl ShowMetrics {
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Basic => "basic",
            Self::Advanced => "advanced",
        }
    }
}

impl fmt::Display for ShowMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for ShowMetrics {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "off" | "0" | "false" => Ok(Self::Off),
            "basic" | "on" | "1" | "true" => Ok(Self::Basic),
            "advanced" | "full" | "2" => Ok(Self::Advanced),
            _ => Err(format!("expected off, basic or advanced, got {s:?}")),
        }
    }
}

/// Frames kept for the graph and the 1% low.
pub const HISTORY: usize = 240;
/// Frames averaged for the headline numbers.
const AVERAGE_OVER: usize = 60;

#[derive(Resource, Debug, Default, Clone)]
pub struct Metrics {
    trace: super::trace::Samples,
    /// Recent frame times in milliseconds, oldest first.
    pub frame_ms: VecDeque<f32>,
    pub adapter: String,
    pub meshes: usize,
    pub textures: usize,
    pub status: String,
    /// The scene's readout (see `Scene::readout`).
    pub hud: String,
    pub output_size: [u32; 2],
    pub render_size: [u32; 2],
    pub work: work::Work,
    pub limit: &'static str,
    pub limit_hint: &'static str,
    pub generation_configured: bool,
    rates: rates::Rates,
    clock: Option<Instant>,
    last_collect: Option<Instant>,
    last_render: u64,
    last_gpu_sample: u64,
    gpu_updated: Option<Instant>,
    work_history: VecDeque<work::Work>,
    gpu_history: VecDeque<f32>,
}

impl Metrics {
    pub fn push(&mut self, dt_secs: f32) {
        if self.frame_ms.len() == HISTORY {
            self.frame_ms.pop_front();
        }
        self.frame_ms.push_back(dt_secs * 1000.0);
    }

    /// Mean frame time of the latest frames, in milliseconds.
    pub fn average_ms(&self) -> f32 {
        let n = self.frame_ms.len().min(AVERAGE_OVER);
        if n == 0 {
            return 0.0;
        }
        self.frame_ms.iter().rev().take(n).sum::<f32>() / n as f32
    }

    pub fn fps(&self) -> f32 {
        let ms = self.average_ms();
        self.rates.rendered().unwrap_or_else(|| if ms > 0.0 { 1000.0 / ms } else { 0.0 })
    }

    pub fn presented_fps(&self) -> Option<f32> {
        self.rates.presented()
    }

    pub fn rate_line(&self) -> String {
        let output = self.presented_fps().map(|fps| format!("{fps:.0}")).unwrap_or_else(|| "…".into());
        format!("Base {:.0} FPS · Presented {output} FPS", self.fps())
    }

    fn sample_work(&mut self, now: Instant, performance: blackbox_gfx::FramePerformance, cpu_update: f32, sleep: f32) {
        if performance.gpu_sample != self.last_gpu_sample {
            self.last_gpu_sample = performance.gpu_sample;
            if let Some(ms) = performance.gpu_render_ms.filter(|v| v.is_finite() && *v > 0.0) {
                self.gpu_history.push_back(ms);
                if self.gpu_history.len() > AVERAGE_OVER {
                    self.gpu_history.pop_front();
                }
                self.gpu_updated = Some(now);
            }
        }
        if performance.rendered_frames != self.last_render {
            self.last_render = performance.rendered_frames;
            self.work_history.push_back(work::Work {
                cpu_ms: cpu_update + performance.cpu_render_ms,
                acquire_ms: performance.acquire_wait_ms,
                present_ms: performance.present_wait_ms,
                sleep_ms: sleep,
                gpu_ms: None,
                activity: performance.activity,
            });
            if self.work_history.len() > AVERAGE_OVER {
                self.work_history.pop_front();
            }
        }
        let n = self.work_history.len().max(1) as f32;
        self.work = work::Work {
            activity: performance.activity,
            cpu_ms: self.work_history.iter().map(|v| v.cpu_ms).sum::<f32>() / n,
            acquire_ms: self.work_history.iter().map(|v| v.acquire_ms).sum::<f32>() / n,
            present_ms: self.work_history.iter().map(|v| v.present_ms).sum::<f32>() / n,
            sleep_ms: self.work_history.iter().map(|v| v.sleep_ms).sum::<f32>() / n,
            gpu_ms: self
                .gpu_updated
                .filter(|last| now.duration_since(*last).as_secs_f32() < 1.0)
                .map(|_| self.gpu_history.iter().sum::<f32>() / self.gpu_history.len() as f32),
        };
    }

    /// The frame rate of the slowest 1% of the recorded frames (at least one frame).
    pub fn low_1pct_fps(&self) -> f32 {
        if self.frame_ms.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f32> = self.frame_ms.iter().copied().collect();
        sorted.sort_by(|a, b| b.total_cmp(a));
        let worst = &sorted[..(sorted.len() / 100).max(1)];
        let ms = worst.iter().sum::<f32>() / worst.len() as f32;
        if ms > 0.0 { 1000.0 / ms } else { 0.0 }
    }

    pub fn worst_ms(&self) -> f32 {
        self.frame_ms.iter().copied().fold(0.0, f32::max)
    }
}

/// Last system of the frame: record this frame's time and refresh the renderer's counts.
pub fn collect(
    mut metrics: ResMut<Metrics>,
    time: Res<Time<Real>>,
    host: NonSend<Host>,
    settings: Res<crate::settings::Settings>,
    window: Single<&bevy_window::Window, With<bevy_window::PrimaryWindow>>,
) {
    let now = Instant::now();
    let dt = metrics.last_collect.replace(now).map_or(time.delta_secs(), |last| now.duration_since(last).as_secs_f32());
    metrics.push(dt);
    let Some(renderer) = host.renderer.as_ref() else { return };
    let stats = renderer.stats();
    let performance = renderer.frame_performance();
    let latency = renderer.latency_status();
    super::trace::observe(&mut metrics.trace, renderer.as_ref(), &host, &settings, window.focused);
    let seconds = now.duration_since(*metrics.clock.get_or_insert(now)).as_secs_f64();
    let rendered = match renderer.info().renderer {
        "blackbox" => performance.rendered_frames,
        _ => u64::from(host.frames),
    };
    metrics.rates.push(seconds, rendered, latency.presented_total, latency.generation_active);
    metrics.generation_configured = latency.generation_active;
    metrics.output_size = renderer.surface_size();
    metrics.render_size = renderer.render_size();
    metrics.sample_work(now, performance, host.cpu_update_ms, host.reflex_sleep_ms);
    let cap_us = settings.max_fps.microseconds();
    let cap = (cap_us > 0 && !settings.vsync).then(|| 1_000_000.0 / cap_us as f32);
    let frame_ms = metrics.rates.rendered().filter(|v| *v > 0.0).map_or(0.0, |fps| 1000.0 / fps);
    let assessment = metrics.work.limit(frame_ms, cap, settings.vsync, window.focused, latency.generation_active);
    metrics.limit = assessment.label;
    metrics.limit_hint = assessment.hint;
    (metrics.meshes, metrics.textures) = (stats.meshes, stats.textures);
    if metrics.adapter.is_empty() {
        metrics.adapter = renderer.info().summary();
    }
    metrics.status = host.scene.status().unwrap_or_default();
    metrics.hud = match settings.show_readout {
        ShowReadout::Off => String::new(),
        level => host.scene.readout(level).unwrap_or_default(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels() {
        assert_eq!("basic".parse::<ShowMetrics>(), Ok(ShowMetrics::Basic));
        assert_eq!("OFF".parse::<ShowMetrics>(), Ok(ShowMetrics::Off));
        assert_eq!("advanced".parse::<ShowMetrics>(), Ok(ShowMetrics::Advanced));
        assert!("loud".parse::<ShowMetrics>().is_err());
        for level in [ShowMetrics::Off, ShowMetrics::Basic, ShowMetrics::Advanced] {
            assert_eq!(level.name().parse::<ShowMetrics>(), Ok(level));
        }
    }

    #[test]
    fn steady_frames() {
        let mut m = Metrics::default();
        for _ in 0..100 {
            m.push(1.0 / 100.0);
        }
        assert!((m.fps() - 100.0).abs() < 0.01);
        assert!((m.low_1pct_fps() - 100.0).abs() < 0.01);
        assert!((m.average_ms() - 10.0).abs() < 1e-3);
    }

    #[test]
    fn one_hitch_drags_the_low_but_not_the_average_much() {
        let mut m = Metrics::default();
        for i in 0..200 {
            m.push(if i == 100 { 0.1 } else { 0.01 });
        }
        // 2 worst of 200 frames: the 100 ms hitch and a 10 ms frame.
        assert!((m.low_1pct_fps() - 1000.0 / 55.0).abs() < 0.1);
        assert!(m.fps() > 90.0);
        assert!((m.worst_ms() - 100.0).abs() < 1e-3);
    }

    #[test]
    fn history_is_bounded() {
        let mut m = Metrics::default();
        for _ in 0..(HISTORY + 50) {
            m.push(0.016);
        }
        assert_eq!(m.frame_ms.len(), HISTORY);
    }
}
