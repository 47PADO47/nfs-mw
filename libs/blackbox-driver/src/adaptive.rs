//! The adaptive PID controller: model-reference adaptive control with the MIT rule.
//! Spec: `docs/specs/ai-driver-control-pid.md` (§1.3, §3.2).

use std::collections::VecDeque;

/// Length of the derivative windows, seconds.
const WINDOW: f32 = 0.1;
/// Seconds each term is tuned before the next one is.
const TIME_SLICE: f32 = 0.1;
const SENSITIVITY_LIMIT: f32 = 1000.0;
const MODEL_ERROR_FLOOR: f32 = 0.001;

/// A window of the last 0.1 s of samples.
#[derive(Debug, Clone, Default)]
struct TimeWindow {
    samples: VecDeque<(f32, f32)>,
}

impl TimeWindow {
    fn record(&mut self, now: f32, value: f32) {
        self.samples.push_back((now, value));
        while self.samples.front().is_some_and(|&(t, _)| now - t > WINDOW) {
            self.samples.pop_front();
        }
    }

    fn mean(&self) -> f32 {
        match self.samples.is_empty() {
            true => 0.0,
            false => self.samples.iter().map(|s| s.1).sum::<f32>() / self.samples.len() as f32,
        }
    }
}

/// Coefficient limits and adaptation settings of the three terms.
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveSettings {
    pub ranges: [(f32, f32); 3],
    pub gamma: f32,
    pub threshold: f32,
}

impl AdaptiveSettings {
    /// The racing set (also used by AI drag opponents).
    pub const RACING: Self = Self { ranges: [(0.4, 1.0), (0.01, 0.1), (0.1, 0.4)], gamma: 1e-5, threshold: 0.01 };
    /// The set of a human car whose steering is taken over in a drag race.
    pub const DRAG: Self = Self { ranges: [(0.6, 1.2), (0.05, 0.5), (0.02, 0.6)], gamma: 1e-5, threshold: 0.01 };
}

#[derive(Debug, Clone)]
pub struct AdaptivePid {
    settings: AdaptiveSettings,
    /// Coefficients of P, I and D.
    coefficients: [f32; 3],
    /// Current term values of P, I and D.
    terms: [f32; 3],
    windows: [TimeWindow; 3],
    model_error_window: TimeWindow,
    last_model_error: f32,
}

impl AdaptivePid {
    pub fn new(settings: AdaptiveSettings) -> Self {
        Self {
            settings,
            coefficients: [0.0; 3],
            terms: [0.0; 3],
            windows: Default::default(),
            model_error_window: TimeWindow::default(),
            last_model_error: 0.0,
        }
    }

    pub fn set_terms(&mut self, p: f32, i: f32, d: f32) {
        self.terms = [p, i, d];
    }

    /// Overwrites the coefficients without clamping.
    pub fn force_coefficients(&mut self, p: f32, i: f32, d: f32) {
        self.coefficients = [p, i, d];
    }

    pub fn coefficients(&self) -> [f32; 3] {
        self.coefficients
    }

    pub fn output(&self) -> f32 {
        self.coefficients.iter().zip(self.terms).map(|(c, t)| c * t.clamp(-99_999.0, 99_999.0)).sum()
    }

    /// One adaptation step. `clock` is the simulation time in seconds.
    pub fn update(&mut self, model: f32, actual: f32, dt: f32, clock: f32) {
        let model_error = actual - model;
        let k = ((clock / TIME_SLICE).floor() as i64).rem_euclid(3) as usize;
        let mut derivative = 0.0;
        if self.terms[k].abs() >= self.settings.threshold {
            let sensitivity = self.sensitivity(k);
            derivative = -self.settings.gamma * model_error * sensitivity;
            let (low, high) = self.settings.ranges[k];
            self.coefficients[k] = (self.coefficients[k] + derivative * dt).clamp(low, high);
        }
        for (i, window) in self.windows.iter_mut().enumerate() {
            window.record(clock, if i == k { derivative } else { 0.0 });
        }
        self.model_error_window.record(clock, (model_error - self.last_model_error) / dt.max(MODEL_ERROR_FLOOR));
        self.last_model_error = model_error;
    }

    fn sensitivity(&self, k: usize) -> f32 {
        let model = self.model_error_window.mean();
        let term = self.windows[k].mean();
        if term.abs() > 1e-9 {
            return (model / term).clamp(-SENSITIVITY_LIMIT, SENSITIVITY_LIMIT);
        }
        match model {
            m if m.abs() < 0.001 => 0.0,
            m if m > 0.0 => 1.0,
            _ => -1.0,
        }
    }
}
