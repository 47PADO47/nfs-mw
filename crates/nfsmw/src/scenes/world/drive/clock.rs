//! Fixed-step timing: the physics runs at 60 Hz whatever the frame rate is.

/// The physics step, seconds.
pub const STEP: f32 = 1.0 / 60.0;
/// Most steps owed after one slow frame; the rest of the time is dropped so the game never spirals.
const MAX_STEPS_PER_FRAME: u32 = 8;

#[derive(Debug, Default)]
pub struct FixedClock {
    accumulator: f32,
}

impl FixedClock {
    /// Add a frame's time; returns how many steps to run now.
    pub fn advance(&mut self, dt: f32) -> u32 {
        self.accumulator += dt.max(0.0);
        let owed = (self.accumulator / STEP) as u32;
        let steps = owed.min(MAX_STEPS_PER_FRAME);
        self.accumulator -= owed as f32 * STEP;
        if owed > steps {
            // The frame took too long: forget the backlog.
            self.accumulator = 0.0;
        }
        steps
    }

    /// How far the display is between the last two steps, 0..1.
    pub fn alpha(&self) -> f32 {
        (self.accumulator / STEP).clamp(0.0, 1.0)
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_follow_the_frame_time() {
        let mut clock = FixedClock::default();
        // 120 fps: a step every other frame.
        let steps: Vec<u32> = (0..6).map(|_| clock.advance(1.0 / 120.0)).collect();
        assert_eq!(steps.iter().sum::<u32>(), 3);
        // 30 fps: two steps per frame.
        assert_eq!(clock.advance(1.0 / 30.0), 2);
    }

    #[test]
    fn keeps_the_remainder_for_interpolation() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(STEP * 1.5), 1);
        assert!((clock.alpha() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn a_stall_does_not_spiral() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(5.0), MAX_STEPS_PER_FRAME);
        assert_eq!(clock.advance(0.0), 0, "the backlog is dropped");
    }

    #[test]
    fn zero_and_negative_time_do_nothing() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(0.0), 0);
        assert_eq!(clock.advance(-1.0), 0);
    }
}
