//! A running average over the last `N` recorded values (the original's `Average`).

use crate::math::finite;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RunningAverage<const N: usize> {
    slots: [f32; N],
    next: usize,
}

impl<const N: usize> Default for RunningAverage<N> {
    fn default() -> Self {
        Self::flushed(0.0)
    }
}

impl<const N: usize> RunningAverage<N> {
    /// All slots hold `value`.
    pub fn flushed(value: f32) -> Self {
        Self { slots: [value; N], next: 0 }
    }

    /// Fills every slot with `value`.
    pub fn flush(&mut self, value: f32) {
        *self = Self::flushed(finite(value));
    }

    /// Replaces the oldest slot with `value`.
    pub fn record(&mut self, value: f32) {
        let value = finite(value);
        self.slots[self.next] = value;
        self.next = (self.next + 1) % N;
    }

    pub fn value(&self) -> f32 {
        self.slots.iter().sum::<f32>() / N as f32
    }
}
