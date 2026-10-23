//! The adaptive display delay (sync-model.md section 3.2).

use std::collections::VecDeque;

use super::InterpConfig;

/// Snapshot ages kept for the transit and jitter estimates: about 3 s at 20 Hz.
const KEPT_AGES: usize = 64;
/// The allowance falls by this many ticks per second when the jitter is gone.
const LOWER_TICKS_PER_S: f64 = 1.0;

/// How far behind the estimated server tick remote entities are drawn, in ticks: the transit (the smallest snapshot age
/// seen lately), the base delay (two snapshot intervals) and the jitter allowance.
#[derive(Debug, Clone)]
pub struct AdaptiveDelay {
    interval: f64,
    base: f64,
    max_allowance: f64,
    /// Snapshot ages at arrival, ticks, oldest first.
    ages: VecDeque<f64>,
    transit: f64,
    jitter_p95: f64,
    allowance: f64,
    /// The delay in use, slewed toward the target.
    current: Option<f64>,
}

impl AdaptiveDelay {
    pub fn new(config: &InterpConfig) -> Self {
        let interval = config.snapshot_interval_ticks.max(1.0);
        Self {
            interval,
            base: interval * config.delay_intervals.max(0.0),
            max_allowance: interval * config.max_allowance_intervals.max(0.0),
            ages: VecDeque::with_capacity(KEPT_AGES),
            transit: 0.0,
            jitter_p95: 0.0,
            allowance: 0.0,
            current: None,
        }
    }

    /// A snapshot arrived `age_ticks` after its tick (the estimated server tick at arrival minus the snapshot's tick).
    /// Non-finite ages are ignored.
    pub fn on_snapshot(&mut self, age_ticks: f64) {
        if !age_ticks.is_finite() {
            return;
        }
        if self.ages.len() >= KEPT_AGES {
            self.ages.pop_front();
        }
        self.ages.push_back(age_ticks);
        self.transit = self.ages.iter().copied().fold(f64::INFINITY, f64::min);
        let mut lateness: Vec<f64> = self.ages.iter().map(|age| age - self.transit).collect();
        lateness.sort_by(f64::total_cmp);
        let rank = ((lateness.len() - 1) as f64 * 0.95).round() as usize;
        self.jitter_p95 = lateness.get(rank).copied().unwrap_or(0.0);
        if self.jitter_p95 > self.interval * 0.5 {
            self.allowance = self.allowance.max(self.jitter_p95.min(self.max_allowance));
        }
        if self.current.is_none() {
            self.current = Some(self.target());
        }
    }

    /// Let `dt_ms` pass: the allowance falls toward what the jitter needs, and the delay in use moves toward its
    /// target so that any change completes in about a second.
    pub fn advance(&mut self, dt_ms: f64) {
        if !dt_ms.is_finite() || dt_ms <= 0.0 {
            return;
        }
        let dt_s = dt_ms / 1000.0;
        let needed = match self.jitter_p95 > self.interval * 0.5 {
            true => self.jitter_p95.min(self.max_allowance),
            false => 0.0,
        };
        if self.allowance > needed {
            self.allowance = (self.allowance - LOWER_TICKS_PER_S * dt_s).max(needed);
        }
        let target = self.target();
        let Some(current) = self.current else {
            return;
        };
        let diff = target - current;
        let step = diff.abs().max(self.interval) * dt_s;
        self.current = Some(current + diff.clamp(-step, step));
    }

    fn target(&self) -> f64 {
        self.transit + self.base + self.allowance
    }

    /// The delay behind the estimated server tick, ticks. Before the first snapshot it is the base delay.
    pub fn delay_ticks(&self) -> f64 {
        self.current.unwrap_or(self.base)
    }

    /// The smallest snapshot age seen lately, ticks.
    pub fn transit_ticks(&self) -> f64 {
        self.transit
    }

    /// The jitter allowance, ticks.
    pub fn allowance_ticks(&self) -> f64 {
        self.allowance
    }

    /// The running 95th percentile of snapshot lateness, ticks.
    pub fn jitter_ticks(&self) -> f64 {
        self.jitter_p95
    }
}
