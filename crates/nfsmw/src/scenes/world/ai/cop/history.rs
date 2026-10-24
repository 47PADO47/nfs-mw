//! Time-resampled target speed history. Spec: ai-pursuit-driving-integration §1.

const RESOLUTION: f32 = 2.0 / 16.0;

#[derive(Debug, Clone)]
pub(super) struct History {
    values: [f32; 16],
    cursor: usize,
    fraction: f32,
    latest: f32,
}

impl Default for History {
    fn default() -> Self {
        Self { values: [0.0; 16], cursor: 0, fraction: 0.0, latest: 0.0 }
    }
}

impl History {
    pub(super) fn hash_into(&self, hash: &mut crate::scenes::world::StateHasher) {
        for value in self.values {
            hash.f32(value);
        }
        hash.u64(self.cursor as u64);
        hash.f32(self.fraction);
        hash.f32(self.latest);
    }
    pub fn reset(&mut self, speed: f32) {
        self.values.fill(speed);
        self.cursor = 0;
        self.fraction = 0.0;
        self.latest = speed;
    }

    pub fn add(&mut self, speed: f32, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let count = ((dt + self.fraction) / RESOLUTION).floor() as usize;
        for crossing in 1..=count {
            let blend = (crossing as f32 * RESOLUTION - self.fraction) / dt;
            self.cursor = (self.cursor + 1) % 16;
            self.values[self.cursor] = self.latest + (speed - self.latest) * blend;
        }
        self.fraction = (self.fraction + dt - count as f32 * RESOLUTION).max(0.0);
        self.latest = speed;
    }

    fn age(&self, offset: usize) -> f32 {
        self.values[(self.cursor + 16 - offset) % 16]
    }

    pub fn sample(&self, delay: f32) -> f32 {
        if delay <= self.fraction {
            if self.fraction < 0.0001 {
                return self.latest;
            }
            return self.age(0) + (self.latest - self.age(0)) * ((self.fraction - delay) / self.fraction);
        }
        let age = (delay - self.fraction) / RESOLUTION;
        let whole = age.floor() as usize;
        if whole >= 15 {
            return self.age(15);
        }
        self.age(whole) + (self.age(whole + 1) - self.age(whole)) * age.fract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resampling_is_independent_of_action_period_and_queries_fractional_age() {
        let mut history = History::default();
        history.reset(10.0);
        history.add(0.0, 0.2);
        assert!((history.sample(0.0) - 0.0).abs() < 0.0001);
        assert!((history.sample(0.1) - 5.0).abs() < 0.0001);
        assert!((history.sample(0.2) - 10.0).abs() < 0.0001);
    }
    #[test]
    fn a_ramp_sampled_at_different_action_rates_has_the_same_history() {
        let mut fast = History::default();
        let mut slow = History::default();
        for step in 1..=120 {
            fast.add(step as f32 / 60.0, 1.0 / 60.0);
        }
        for step in 1..=15 {
            slow.add(step as f32 * 8.0 / 60.0, 8.0 / 60.0);
        }
        for delay in [0.0, 0.1, 0.5, 1.5, 1.75] {
            assert!((fast.sample(delay) - slow.sample(delay)).abs() < 0.0001, "{delay}");
        }
    }
}
