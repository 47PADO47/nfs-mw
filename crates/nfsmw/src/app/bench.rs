//! `--bench-seconds N`: measure frame times for N seconds once the scene is loaded, print them with the renderer's
//! name and the process's CPU time and memory, and exit. It works with every renderer, so numbers compare.
//!
//! Run it with `--no-vsync` and `--max-fps unlocked`, or the frame time is the monitor's refresh interval.

use std::time::{Duration, Instant};

/// Frames this long after the scene is ready are not counted, so a first-use hitch does not skew the percentiles.
const WARMUP: Duration = Duration::from_secs(2);
/// How long to wait for the scene to report that it is ready before measuring anyway.
const READY_TIMEOUT: Duration = Duration::from_secs(120);

/// What a run measured.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub frames: usize,
    pub seconds: f64,
    /// Frame times in milliseconds.
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub worst: f64,
    /// Process CPU time (user + system) per frame in milliseconds, or `None` where the OS does not say.
    pub cpu_ms_per_frame: Option<f64>,
    /// Resident memory at the end, and its peak, in MiB.
    pub rss_mib: Option<f64>,
    pub peak_rss_mib: Option<f64>,
}

/// `p` (0 to 1) of an already sorted list, by nearest rank.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// The frame-time statistics of `times` (milliseconds).
pub fn summarize(times: &[f64], seconds: f64) -> Report {
    let mut sorted = times.to_vec();
    sorted.sort_by(f64::total_cmp);
    Report {
        frames: times.len(),
        seconds,
        p50: percentile(&sorted, 0.50),
        p95: percentile(&sorted, 0.95),
        p99: percentile(&sorted, 0.99),
        worst: sorted.last().copied().unwrap_or(0.0),
        cpu_ms_per_frame: None,
        rss_mib: None,
        peak_rss_mib: None,
    }
}

/// User plus system CPU seconds of this process (Linux), from `/proc/self/stat`.
fn cpu_seconds() -> Option<f64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // Fields after the parenthesised name: state is 3, utime 14, stime 15 (clock ticks, 100 per second).
    let rest = &stat[stat.rfind(')')? + 2..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let ticks = |i: usize| fields.get(i)?.parse::<f64>().ok();
    Some((ticks(11)? + ticks(12)?) / 100.0)
}

/// A `Vm...` line of `/proc/self/status` in MiB (Linux).
fn status_mib(key: &str) -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with(key))?;
    let kib: f64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib / 1024.0)
}

pub struct Bench {
    duration: Duration,
    created: Instant,
    ready_at: Option<Instant>,
    measuring_since: Option<Instant>,
    last: Instant,
    times: Vec<f64>,
    cpu_start: Option<f64>,
}

impl Bench {
    pub fn new(seconds: f32) -> Self {
        let now = Instant::now();
        Self {
            duration: Duration::from_secs_f32(seconds.max(0.1)),
            created: now,
            ready_at: None,
            measuring_since: None,
            last: now,
            times: Vec::new(),
            cpu_start: None,
        }
    }

    /// Call once per finished frame with whether the scene is ready. Returns the report once the time is up.
    pub fn frame(&mut self, scene_ready: bool) -> Option<Report> {
        let now = Instant::now();
        let dt = now - self.last;
        self.last = now;
        if scene_ready || now - self.created > READY_TIMEOUT {
            self.ready_at.get_or_insert(now);
        }
        let ready_at = self.ready_at?;
        if now - ready_at < WARMUP {
            return None;
        }
        let since = *self.measuring_since.get_or_insert_with(|| {
            self.cpu_start = cpu_seconds();
            now
        });
        self.times.push(dt.as_secs_f64() * 1000.0);
        if now - since < self.duration {
            return None;
        }
        let seconds = (now - since).as_secs_f64();
        let mut report = summarize(&self.times, seconds);
        report.cpu_ms_per_frame =
            self.cpu_start.zip(cpu_seconds()).map(|(a, b)| (b - a) * 1000.0 / self.times.len() as f64);
        report.rss_mib = status_mib("VmRSS:");
        report.peak_rss_mib = status_mib("VmHWM:");
        Some(report)
    }
}

impl Report {
    /// The lines printed at exit.
    pub fn lines(&self, renderer: &str) -> Vec<String> {
        let opt = |v: Option<f64>| v.map_or_else(|| "n/a".to_owned(), |v| format!("{v:.2}"));
        vec![
            format!(
                "bench: renderer {renderer}, {} frames in {:.1} s ({:.1} fps)",
                self.frames,
                self.seconds,
                self.frames as f64 / self.seconds.max(1e-9)
            ),
            format!(
                "bench: frame time ms p50 {:.2} p95 {:.2} p99 {:.2} worst {:.2}",
                self.p50, self.p95, self.p99, self.worst
            ),
            format!(
                "bench: cpu ms/frame {} rss MiB {} peak rss MiB {}",
                opt(self.cpu_ms_per_frame),
                opt(self.rss_mib),
                opt(self.peak_rss_mib)
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_use_the_nearest_rank() {
        let sorted: Vec<f64> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&sorted, 0.50), 50.0);
        assert_eq!(percentile(&sorted, 0.95), 95.0);
        assert_eq!(percentile(&sorted, 0.99), 99.0);
        assert_eq!(percentile(&sorted, 1.0), 100.0);
        assert_eq!(percentile(&[], 0.5), 0.0);
        assert_eq!(percentile(&[7.0], 0.99), 7.0);
    }

    #[test]
    fn the_summary_sorts_and_finds_the_worst_frame() {
        let report = summarize(&[16.0, 40.0, 15.0, 16.5], 1.0);
        assert_eq!(report.frames, 4);
        assert_eq!(report.worst, 40.0);
        assert_eq!(report.p50, 16.0);
    }

    #[test]
    fn nothing_is_measured_before_the_scene_is_ready() {
        let mut bench = Bench::new(0.1);
        assert!(bench.frame(false).is_none());
        assert!(bench.times.is_empty());
    }

    #[test]
    fn the_report_names_the_renderer_and_the_percentiles() {
        let lines = summarize(&[10.0, 20.0], 1.0).lines("bevy");
        assert!(lines[0].contains("renderer bevy") && lines[1].contains("p99"), "{lines:?}");
        assert!(lines[2].contains("n/a"));
    }
}
