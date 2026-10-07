//! Frame statistics, as plain data. The overlay draws them; nothing here knows about egui.

use std::collections::VecDeque;
use std::fmt;
use std::str::FromStr;

use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};

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
    /// Recent frame times in milliseconds, oldest first.
    pub frame_ms: VecDeque<f32>,
    pub adapter: String,
    pub meshes: usize,
    pub textures: usize,
    pub status: String,
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
        if ms > 0.0 { 1000.0 / ms } else { 0.0 }
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
pub fn collect(mut metrics: ResMut<Metrics>, time: Res<Time<Real>>, host: NonSend<Host>) {
    metrics.push(time.delta_secs());
    let Some(renderer) = host.renderer.as_ref() else { return };
    (metrics.meshes, metrics.textures) = renderer.resource_counts();
    if metrics.adapter.is_empty() {
        metrics.adapter = renderer.adapter_summary();
    }
    metrics.status = host.scene.status().unwrap_or_default();
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
