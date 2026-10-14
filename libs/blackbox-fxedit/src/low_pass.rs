//! Retained software voice cutoff. Spec: `voice-low-pass.md`.
use crate::math;

#[derive(Default)]
pub struct LowPass {
    active: bool,
    previous: f32,
    feedback: f32,
    feed: f32,
}

impl LowPass {
    /// Retarget an integer cutoff; bypass removes history, retargeting retains it.
    pub fn set_cutoff(&mut self, rate: u32, cutoff: u32) {
        let normalized = (f64::from(cutoff) / (f64::from(rate.max(1)) * 0.5)) as f32;
        if normalized >= 1.0 {
            self.active = false;
            self.previous = 0.0;
            return;
        }
        if !self.active {
            self.previous = 0.0;
        }
        self.active = true;
        let quantized = (f64::from(rate) * f64::from(normalized)) as f32;
        let numerator = f64::from(quantized.round_ties_even());
        (self.feedback, self.feed) = math::one_pole(numerator, f64::from(rate.max(1)), 1.0);
    }

    /// Process one channel's block, retaining f32 history at the block boundary.
    pub fn process<'a>(&mut self, samples: impl Iterator<Item = &'a mut f32>) {
        if !self.active {
            return;
        }
        let mut previous = f64::from(self.previous);
        for sample in samples {
            previous = f64::from(self.feed) * f64::from(*sample) + previous * f64::from(self.feedback);
            *sample = previous as f32;
        }
        self.previous = previous as f32;
    }
}

#[cfg(test)]
mod tests;
