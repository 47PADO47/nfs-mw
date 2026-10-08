//! Time-based interpolators with the original's behaviour (`cInterpLine`): a value that moves from a start to
//! a finish over a number of milliseconds of accumulated frame time, optionally with a "live" finish that is
//! replaced every update so the value lands wherever the physics is.

use crate::math::{equal_power_sq, finite};

/// Shape of an interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Curve {
    #[default]
    Linear,
    /// `sin^2(pi/2 x)`.
    EqPowerSq,
}

/// One interpolation. A fresh or finished one holds its last value and reports `finished()`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Interp {
    start: f32,
    finish: f32,
    length: f32,
    curve: Curve,
    elapsed: f32,
    value: f32,
    complete: bool,
}

impl Default for Interp {
    fn default() -> Self {
        Self::holding(0.0)
    }
}

impl Interp {
    /// A finished interpolation that holds `value`.
    pub fn holding(value: f32) -> Self {
        Self { start: value, finish: value, length: 0.01, curve: Curve::Linear, elapsed: 0.0, value, complete: true }
    }

    /// Starts moving from `start` to `finish` over `length_ms` (0 or less is read as 10 ms).
    pub fn begin(&mut self, start: f32, finish: f32, length_ms: f32, curve: Curve) {
        let length = finite(length_ms) / 1000.0;
        self.length = if length <= 0.0 { 0.01 } else { length };
        self.start = finite(start);
        self.finish = finite(finish);
        self.curve = curve;
        self.elapsed = 0.0;
        self.value = self.start;
        self.complete = false;
    }

    /// Advances by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if self.complete {
            return;
        }
        self.elapsed += finite(dt).max(0.0);
        if self.elapsed > self.length {
            self.complete = true;
            self.value = self.finish;
            return;
        }
        let x = self.elapsed / self.length;
        let shaped = match self.curve {
            Curve::Linear => x,
            Curve::EqPowerSq => equal_power_sq(x),
        };
        self.value = self.start + (self.finish - self.start) * shaped;
    }

    /// Replaces the finish by `target`, advances, and lands exactly on `target` once finished.
    pub fn update_live(&mut self, dt: f32, target: f32) {
        self.finish = finite(target);
        self.update(dt);
        if self.complete {
            self.value = self.finish;
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn finished(&self) -> bool {
        self.complete
    }
}
