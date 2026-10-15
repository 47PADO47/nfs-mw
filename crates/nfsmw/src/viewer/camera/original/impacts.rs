//! Camera-action impact decay; inputs are closing velocity, not impulse.

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    World,
    Ground,
    Object,
}

#[derive(Debug, Clone, Default)]
pub struct Impacts {
    timers: [f32; 3],
}

impl Impacts {
    pub fn register(&mut self, kind: Kind, closing_speed: f32) {
        let (index, threshold, scale) = match kind {
            Kind::World => (0, 5.0, 0.025),
            Kind::Ground => (1, 3.0, 0.05),
            Kind::Object => (2, 20.0, 0.01),
        };
        if closing_speed <= threshold {
            return;
        }
        let seconds = ((closing_speed - threshold) * scale).min(1.0) * 2.0;
        self.timers[index] = self.timers[index].max(seconds);
    }

    pub fn advance(&mut self, dt: f32) -> [f32; 3] {
        for timer in &mut self.timers {
            *timer = (*timer - dt).max(0.0);
        }
        self.normalized()
    }

    pub fn normalized(&self) -> [f32; 3] {
        self.timers.map(|timer| timer / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_collision_thresholds_and_max_retention() {
        let mut impacts = Impacts::default();
        for (kind, speed, index, expected) in [
            (Kind::World, 5.0, 0, 0.0),
            (Kind::World, 25.0, 0, 0.5),
            (Kind::Ground, 3.0, 1, 0.0),
            (Kind::Ground, 13.0, 1, 0.5),
            (Kind::Object, 20.0, 2, 0.0),
            (Kind::Object, 70.0, 2, 0.5),
        ] {
            impacts.register(kind, speed);
            assert!((impacts.normalized()[index] - expected).abs() < 1e-6);
        }
        impacts.register(Kind::World, 10.0);
        assert_eq!(impacts.normalized(), [0.5; 3]);
        assert_eq!(impacts.advance(0.25), [0.375; 3]);
        assert_eq!(impacts.advance(1.0), [0.0; 3]);
    }
}
