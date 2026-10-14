//! Retain historical class words while bounded catch-up repays elapsed time.
use std::collections::VecDeque;

const STEP: f64 = 1.0 / 60.0;
const EPSILON: f64 = 1e-7;

#[derive(Default)]
pub(in crate::audio) struct Timeline {
    submitted: f64,
    consumed: u64,
    segments: VecDeque<(f64, Vec<i32>)>,
}

impl Timeline {
    pub fn submitted(&self) -> f64 {
        self.submitted
    }
    pub fn pending(&self) -> f64 {
        (self.submitted - self.consumed as f64 * STEP).max(0.0)
    }
    pub fn push(&mut self, dt: f32, params: &[i32]) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.submitted += f64::from(dt);
        if let Some(last) = self.segments.back_mut()
            && last.1 == params
        {
            last.0 = self.submitted;
            return;
        }
        self.segments.push_back((self.submitted, params.to_vec()));
    }
    pub fn next(&mut self) -> Option<(f64, Vec<i32>)> {
        let at = (self.consumed + 1) as f64 * STEP;
        if at > self.submitted + EPSILON {
            return None;
        }
        while self.segments.len() > 1 && self.segments.front().is_some_and(|s| s.0 + EPSILON < at) {
            self.segments.pop_front();
        }
        let params = self.segments.front()?.1.clone();
        self.consumed += 1;
        Some((at, params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catch_up_uses_historical_parameters_before_new_input() {
        let mut clock = Timeline::default();
        clock.push(1.0, &[10]);
        for _ in 0..12 {
            assert_eq!(clock.next().unwrap().1, [10]);
        }
        clock.push(1.0 / 60.0, &[20]);
        for _ in 12..60 {
            assert_eq!(clock.next().unwrap().1, [10]);
        }
        assert_eq!(clock.next().unwrap().1, [20]);
        assert!(clock.next().is_none());
    }
    #[test]
    fn all_render_rates_deliver_the_same_parameter_sequence() {
        let reference: Vec<_> = (0..180).map(|i| vec![i / 30]).collect();
        for fps in [5, 30, 60, 120, 144, 240, 360] {
            let mut clock = Timeline::default();
            let mut actual = Vec::new();
            // Split at actual producer changes, even when render frames span several changes.
            for section in 0..6 {
                let mut remaining = 0.5_f64;
                while remaining > 1e-8 {
                    let dt = remaining.min(1.0 / fps as f64);
                    clock.push(dt as f32, &[section]);
                    for _ in 0..12 {
                        if let Some((_, p)) = clock.next() {
                            actual.push(p);
                        }
                    }
                    remaining -= dt;
                }
            }
            while let Some((_, p)) = clock.next() {
                actual.push(p);
            }
            assert_eq!(actual, reference, "{fps} FPS");
            assert!(clock.pending() < 1e-6);
        }
    }
}
