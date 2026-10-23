//! Correction statistics: how far and how often the own car is corrected (features.md section 4, MP-82).

use std::collections::VecDeque;

/// Correction distances kept for the percentiles.
const KEPT_DISTANCES: usize = 1024;
/// The rolling window of the per-minute rates, milliseconds.
const MINUTE_MS: u64 = 60_000;
/// Most event times kept for the per-minute rates, whatever the rate.
const KEPT_TIMES: usize = 4096;

/// What the reconciler did over time. Memory is bounded: the last 1,024 correction distances and at most a minute of
/// event times.
#[derive(Debug, Clone, Default)]
pub struct CorrectionStats {
    /// Distances of the last corrections and snaps, metres, oldest first.
    distances: VecDeque<f32>,
    correction_times: VecDeque<u64>,
    snap_times: VecDeque<u64>,
    checks: u64,
    corrections: u64,
    snaps: u64,
    largest: f32,
}

impl CorrectionStats {
    /// A server state was compared with the prediction.
    pub fn record_check(&mut self) {
        self.checks += 1;
    }

    /// A smoothed correction of `distance` metres at `now_ms`.
    pub fn record_correction(&mut self, distance: f32, now_ms: u64) {
        self.corrections += 1;
        self.push_distance(distance);
        push_time(&mut self.correction_times, now_ms);
    }

    /// A hard snap of `distance` metres at `now_ms`.
    pub fn record_snap(&mut self, distance: f32, now_ms: u64) {
        self.snaps += 1;
        self.push_distance(distance);
        push_time(&mut self.snap_times, now_ms);
    }

    fn push_distance(&mut self, distance: f32) {
        let distance = match distance.is_finite() {
            true => distance.max(0.0),
            false => f32::MAX,
        };
        self.largest = self.largest.max(distance);
        if self.distances.len() >= KEPT_DISTANCES {
            self.distances.pop_front();
        }
        self.distances.push_back(distance);
    }

    /// Server states compared so far.
    pub fn checks(&self) -> u64 {
        self.checks
    }

    /// Smoothed corrections so far.
    pub fn corrections(&self) -> u64 {
        self.corrections
    }

    /// Hard snaps so far.
    pub fn snaps(&self) -> u64 {
        self.snaps
    }

    /// The largest correction or snap so far, metres.
    pub fn largest(&self) -> f32 {
        self.largest
    }

    /// The `p`-th percentile (0..=1) of the kept correction and snap distances, metres, nearest rank; `None` before
    /// the first correction.
    pub fn percentile(&self, p: f32) -> Option<f32> {
        if self.distances.is_empty() {
            return None;
        }
        let mut sorted: Vec<f32> = self.distances.iter().copied().collect();
        sorted.sort_by(f32::total_cmp);
        let rank = (p.clamp(0.0, 1.0) * (sorted.len() - 1) as f32).round() as usize;
        sorted.get(rank).copied()
    }

    /// The median correction distance, metres.
    pub fn median(&self) -> Option<f32> {
        self.percentile(0.5)
    }

    /// The 95th percentile correction distance, metres.
    pub fn p95(&self) -> Option<f32> {
        self.percentile(0.95)
    }

    /// Smoothed corrections in the minute before `now_ms`.
    pub fn corrections_per_minute(&self, now_ms: u64) -> usize {
        in_last_minute(&self.correction_times, now_ms)
    }

    /// Hard snaps in the minute before `now_ms`.
    pub fn snaps_per_minute(&self, now_ms: u64) -> usize {
        in_last_minute(&self.snap_times, now_ms)
    }
}

fn push_time(times: &mut VecDeque<u64>, now_ms: u64) {
    while times.front().is_some_and(|t| now_ms.saturating_sub(*t) >= MINUTE_MS) {
        times.pop_front();
    }
    if times.len() >= KEPT_TIMES {
        times.pop_front();
    }
    times.push_back(now_ms);
}

fn in_last_minute(times: &VecDeque<u64>, now_ms: u64) -> usize {
    times.iter().filter(|t| now_ms.saturating_sub(**t) < MINUTE_MS && **t <= now_ms).count()
}
