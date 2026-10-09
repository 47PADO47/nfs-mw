//! The history of one error signal.
//! Spec: `docs/specs/ai-driver-control-pid.md` (§1.2).

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PidError {
    integral_len: usize,
    derivative_len: usize,
    frequency: f32,
    current: f32,
    times: VecDeque<f32>,
    areas: VecDeque<f32>,
    slopes: VecDeque<f32>,
}

impl PidError {
    pub fn new(integral_len: usize, derivative_len: usize, frequency: f32) -> Self {
        Self {
            integral_len,
            derivative_len,
            frequency,
            current: 0.0,
            times: VecDeque::new(),
            areas: VecDeque::new(),
            slopes: VecDeque::new(),
        }
    }

    /// Records error `e`, `t` seconds after the previous record.
    pub fn record(&mut self, e: f32, t: f32) {
        let (prev, t) = (self.current, t.max(1e-6));
        self.current = e;
        let delta = e - prev;
        push(&mut self.times, t, self.integral_len);
        push(&mut self.areas, t * (prev + delta / 2.0), self.integral_len);
        push(&mut self.slopes, delta / t, self.derivative_len);
    }

    pub fn error(&self) -> f32 {
        self.current
    }

    /// The sum of the recorded areas scaled by the sample count over `frequency * sum(times)`. With a
    /// constant step this is the sum of the last errors over the frequency, not a true time integral.
    pub fn integral(&self) -> f32 {
        let time: f32 = self.times.iter().sum();
        if self.areas.is_empty() || time <= 0.0 {
            return 0.0;
        }
        self.areas.iter().sum::<f32>() * self.areas.len() as f32 / (self.frequency * time)
    }

    pub fn derivative(&self) -> f32 {
        match self.slopes.is_empty() {
            true => 0.0,
            false => self.slopes.iter().sum::<f32>() / self.slopes.len() as f32,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.integral_len, self.derivative_len, self.frequency);
    }
}

fn push(ring: &mut VecDeque<f32>, value: f32, capacity: usize) {
    if ring.len() == capacity {
        ring.pop_front();
    }
    ring.push_back(value);
}
